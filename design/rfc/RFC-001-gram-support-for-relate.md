---
id: RFC-001
title: Gram Graph Support for Relate
status: review
created: 2026-11-06
adrs: [ADR-001, ADR-002, ADR-003]
---

# RFC-001: Gram Graph Support for Relate

## 1. Summary

Gram is a plain-text notation for graphs and the patterns built from them. A gram file writes nodes as `(a:Person {name: "Ada"})`, relationships as `(a)-[:KNOWS]->(b)`, and chains of both as paths, so a small property graph can be read, diffed and edited by hand. Relate is an embeddable, serverless graph database queried with Cypher. Its WebAssembly build can render query results as gram text, but Relate cannot load a gram file, and it has no way to write a database back out as one.

`relate-gram` is the proposed extension to Relate that adds gram as a two-way format for graph data: load a gram file into an in-memory Relate database, and export a database to a gram file. The organizing claim is that the supported notation should be a strict subset of gram, namely the part that maps directly onto Cypher's property graph model: nodes, relationships, paths and the file's top-level record (a record of properties, at the start of a gram file, describing the file as a whole). Annotations and pattern notation, which have no Cypher counterpart, are out of scope.

The hard part of the work is element identity. Gram writes an identifier on every element so that other parts of the file can refer to it; a property graph database assigns its own internal identifiers and gives users no stable handle on them. Identifiers are used to resolve references on load and are generated or copied on export, several graphs can share one database as separate named graphs, and the top-level record is kept as a "meta node". Details follow in later RFCs and ADRs.

## 2. Problem / Motivation

**Graphs move between tools as scripts or machine formats, and neither can be read or reviewed as a graph.** Relate's current paths for getting graph data in and out are:

- `LOAD CSV`, which brings in rows, so every relationship has to be rebuilt by a Cypher statement that matches endpoints by some key column.
- JSON and Parquet snapshots, which are exact and complete but are machine formats for a whole database, not something to hand-edit or review in a diff.
- `--mode cypher`, which renders result values as literals. For `RETURN p.name AS name, p.age AS age` it prints `CREATE ({name: 'Alice', age: 30})`, one statement per row, with no relationship wiring.
- The WebAssembly build's `renderResult`, which renders a result that holds nodes as gram text. This direction exists, but only for display; nothing reads gram back in.

A gram file is a single artifact that holds nodes, relationships and their properties together, in a form people can diff and review. Without loading and exporting it, such a file cannot become a Relate database, and a Relate database cannot be handed to anyone as one.

**Identity gets in the way of round trips.** A gram file refers to elements by identifier, which is how `(a)-->(b)` and a later `(b)-->(c)` mean the same `b`. Relate's element identifiers are internal. A loader has to decide what a gram identifier means once the data is in the database, and an exporter has to invent identifiers when none are available. If these choices are left implicit, two loads of the same file can disagree, a second load can silently duplicate the first, and an export cannot be loaded back into an equivalent graph.

**A database holds one undifferentiated graph.** Consider two revisions of one dataset, each a small graph of people and their relationships. Loading both into one Relate database today produces a single graph in which every person appears twice, nothing records which revision an element came from, and nothing says that the two copies of a person are the same person.

## 3. Goals

3.1. Load a gram file into an in-memory Relate database.

3.2. Export a Relate database to a gram file.

3.3. Support only the gram constructs that map directly onto Cypher's property graph model (nodes, relationships, paths and the top-level record), and reject every other construct that does not map directly, with an error that locates the offending text.

3.4. Make identity behavior explicit and configurable, with defaults that need no configuration:
  - On load, gram identifiers resolve references within the file and are then dropped.
  - On export, identifiers come from an `@id` value, are copied from a property whose values are unique, or are generated.

3.5. Preserve a gram file's top-level record by loading it as a distinguished "meta node".

3.6. Allow several graphs to live in one database as distinct named graphs. An element's identity within its named graph is kept in a reserved `@id` property that survives a round trip through gram by becoming the element's identifier.

3.7. Load multiple graphs in managed fashion, each into its own named graph, and merge graphs loaded into the same named graph.

3.8. Map property values between gram and Cypher by a documented rule. The first phase preserves only Cypher's own types. A later, advanced mapping may record metadata about properties, so that information gram can express but Cypher cannot store (for example, that a string is markdown) survives a round trip.

3.9. A first release provides load and export through the `relate` command-line tool. Deeper CLI integration, identity-aware language bindings and graph-shaped client result structures are longer-term goals, specified later.

## 4. Non-Goals

4.1. Gram annotations and pattern notation (subject patterns, nested and recursive structure). Neither has a counterpart in Cypher's data model.

4.2. In the first phase, gram values that Cypher cannot store: nested maps and values with no Cypher type, such as ranges and measurements. Retaining them through metadata is a later direction.

4.3. Gram as a durable storage format for Relate. The extension is an import and export format on top of the engine's own storage.

4.4. Query-language support. Queries remain Cypher.

4.5. Changing the Relate engine. The extension works through Relate's public interfaces.

4.6. Language bindings for TypeScript and Python, and client-side result structures, in the first phase. They are longer-term goals (3.9) and are specified later.

## 5. Proposed Approach (high-level)

### 5.1 A strict subset defined by Cypher

The boundary of what is supported is set by one rule: a gram construct is accepted if and only if it maps directly onto Cypher's property graph model. Nodes carry zero or more labels and a property map. A relationship connects two nodes, carries exactly one type and a property map, and has a direction. A path is shorthand for a chain of nodes and relationships. Property values are limited to what becomes a Cypher value: strings, integers, floats, booleans, homogeneous lists of those, and the other types Cypher stores that gram can reach through tagged strings.

The detailed mapping between gram values and Cypher values, including how alternative spellings are converted, is its own topic, taken up in RFC-002. Its first phase preserves Cypher types only; a later phase may record per-property metadata, for instance on the meta node, so that a property known to hold markdown can be written back as a tagged string.

Anything outside this rule is a load error, reported with its position in the file. Silently dropping or approximating a construct would make a loaded database differ from its source without notice. The one planned exception, dropping an unrecognized tag from a tagged string (RFC-002), is reported to the user.

Constructs that carry schema or pattern intent in the wider gram ecosystem are rejected for the same reason: double-colon labels and property slots (`::`), which the pattern-rs schema work uses to mark type slots; fat-arrow and other non-thin relationship arrows, which it reserves for schema and ontology intent; and annotations. Accepting them as ordinary labels, properties or arrows would erase that meaning.

The parser is expected to come from the existing gram tooling rather than being written afresh. pattern-rs's `gram-codec` offers a hand-written parser whose value model lacks maps, measurements and hexadecimal and octal literals, and an optional parser built on tree-sitter-gram that returns source positions but pins an older grammar release and does not expose the arrow family. Which parser to use and what it needs to expose are left to an ADR.

### 5.2 Identity on load

Gram identifiers exist to say "this element, again." On load they are used for exactly that, and then dropped:

- The first appearance of an identifier fixes whether it names a node or a relationship. Using the same identifier later as the other kind is an error.
- A later appearance of an identifier refers to the same element. By default the appearance merges into the element already loaded. The merge rules (what happens to differing property values, additional labels) are left to a later decision.
- Elements written without an identifier cannot be referred to again, so each appearance is a separate element. `(a)-->(b)` written twice, with no identifier on the relationship, yields two relationships.
- After loading, the identifier is not stored on the element. The database's own internal identity applies.

Loading the same file twice into a fresh database each time therefore yields equivalent graphs, and a file that mentions an element in several places yields one element.

### 5.3 Identity on export

An exported file needs an identifier on each element that is referenced more than once. Export offers a small set of strategies:

- Use an element's `@id` value, when it has one (see 5.5).
- Copy the value of a configured property, when every element of that kind has a unique value for it. If some elements lack the property or the values collide, export fails and lists the offending elements.
- Otherwise generate identifiers, numbered in a deterministic order so that exports of the same database are stable and diffable. The naming scheme is left to an ADR.

The default behavior chooses among these in the order listed. Identifiers that are not plain gram symbols are quoted.

### 5.4 The top-level record and the meta node

A gram file may open with a record of properties describing the file as a whole. A property graph has no place for data about the graph itself, so the extension loads the record as a "meta node": a node of a reserved kind that is distinguishable from data nodes, holds the record's properties, and is written back as the top-level record on export. The reserved label, property layout and rules for excluding the meta node from ordinary queries are left to a later decision. In the pattern-rs ecosystem the record's `kind` property says what a file is (for example `{ kind: "schema" }`), so loading a file whose `kind` is not a data graph deserves its own handling (see 7.13).

### 5.5 Named graphs

A single Relate database can hold several graphs, each one a named graph. (Relate itself has three fixed built-in namespaces: the user data graph, the system database and the ontology graph. Named graphs are unrelated to these and live inside the user data graph.) An element's identity within its named graph is stored in a reserved `@id` property. On export, that value becomes the element's gram identifier, so identity survives the round trip: a graph exported from one database and loaded into another as the same named graph lines up with what is already there.

Two operations build on this:

- *Managed load of several graphs.* Each file loads into its own named graph, and the named graph is a first-class thing that can be listed, exported on its own, or removed.
- *Merging.* Loading a graph into a named graph that already exists merges elements whose `@id` values match, using the same merge defaults as in-file repetition (5.2).

The spelling of `@id` values, how a named graph is named, and how both appear in gram are left to a later decision.

### 5.6 Integration with Relate

The extension is a Rust library crate that depends on Relate's public library interface. Its first integration with the `relate` command-line tool is minimal: a way to run load and export. It is developed as a standalone project and may later be brought into the Relate project itself.

Identity-aware libraries for TypeScript and Python, and client-side data structures that carry element identity for tabular and graph-shaped results, are part of the intended direction. Their design is deferred until the core loader, exporter and CLI integration exist.

### 5.7 Phasing

Basic capability comes first: load and export of the strict subset, with the default identity behavior. Named graphs, managed multi-graph loading and merging come second, since they depend on the identity model being settled. Language bindings and client structures come last.

## 6. Alternatives Considered

6.1. **Keep using Cypher scripts as the interchange format.** Rejected: a script says how to build a graph, not what it is. It cannot be reviewed as a graph, and its variables are scoped to one execution, so it cannot be merged with another script's elements or loaded into a named graph.

6.2. **Use the existing JSON or Parquet snapshots.** Rejected for interchange: they are complete and exact for a whole database, but are machine formats that cannot be reviewed or hand-edited as a graph, and do not support loading several graphs into separate named graphs.

6.3. **Support the full gram language.** Rejected: annotations and pattern notation have no meaning in Cypher's data model. Supporting them would mean inventing a mapping, and files that use them would not round-trip.

6.4. **Store gram identifiers on elements as an ordinary property by default.** Rejected as the default: identifiers in a file are scoped to the file, and persisting them would make a second load duplicate the first. The reserved `@id` property provides persisted identity as an explicit choice in named graphs.

6.5. **Make gram a storage format (a snapshot codec) for Relate.** Rejected for now: the strict subset cannot carry everything a database holds, such as indexes, schema and vector types, so it does not suit a durable form. It remains possible to revisit if gram's coverage grows.

6.6. **Contribute directly to Relate rather than building a standalone extension.** Deferred: a standalone project can move faster while the identity model is settled, and the code can be brought into Relate afterward.

6.7. **Another existing graph interchange format (GraphML, DOT, node-link JSON).** Rejected: gram combines labelled nodes, typed relationships and properties in a compact, hand-editable form, and has existing parser tooling in the pattern-rs ecosystem.

## 7. Open Questions

7.1. _Partly resolved by ADR-003._ The meta node carries the reserved label `_GramMeta`, holds the record's properties as ordinary properties, is not hidden from queries, and is written back first on export. Still open: whether it carries the identity of a named graph, which the named-graph decision settles.

7.2. What is the format of an `@id` value, and how is a named graph named and written in gram? Is naming a graph a single prefix on an identifier, a separate property on each element, or something else?

7.3. _Resolved by ADR-001 (§2.3)._ Repeated appearances merge: labels union, properties union by key, and differing values are an error unless `on-conflict` selects `first` or `last`.

7.4. _Resolved by ADR-001 (§2.4)._ A relationship with an identifier keeps its type and endpoints from its first appearance, and a repeat must match them.

7.5. _Partly resolved by ADR-001 (§2.5)._ Elements without an identifier never merge. Still open: is an opt-in merge key needed, so that `(a)-[:R]->(b)` written twice can mean one relationship?

7.6. _Resolved by ADR-001 (§2.6)._ Each element of a path is an ordinary appearance, and a reused inner identifier names the same node.

7.7. How are property values mapped between gram and Cypher, and how might later metadata extend that mapping? (Taken up in RFC-002.)

7.8. Gram has `-->`, `<--`, `--` and bidirectional arrows, other arrow families, and relationships without a type. Cypher relationships are directed and typed. Arrow families other than the thin arrow are rejected, since pattern-rs reserves them for schema intent (5.1). Still open: how undirected and bidirectional thin forms are handled, and whether an untyped relationship is an error.

7.9. Is loading into a non-empty or persistent database in scope, or only into a fresh in-memory one? If it is in scope, what does a load merge with, and how is a loaded in-memory database persisted afterward?

7.10. How much of a gram file's text survives a load and export, beyond its graph? Do comments, element ordering and layout matter?

7.11. What does the first `relate` CLI integration look like (verbs, flags, defaults), and what does it need from Relate's extension mechanism?

7.12. _Resolved by the strict-subset rule (5.1)._ The double-colon before a label or after a property key is rejected, since the pattern-rs schema work uses it to mark type slots and accepting it as an ordinary colon would erase that meaning.

7.13. How does the loader treat a file whose top-level record has a `kind` that does not describe a data graph (for example `schema`)? Loading it as data would produce a graph of schema archetypes with their constraint tags dropped. Is it an error, a warning, or accepted?

7.14. pattern-rs has a canonical gram format (the `pato fmt` command). Should exports match it, or reuse the codec's serializer, so that an exported file is already formatted?

## 8. ADRs

- [ADR-001](../adr/ADR-001-load-identity-and-merge.md): Load Identity and Merge Semantics
- [ADR-002](../adr/ADR-002-export-identity.md): Export Identity
- [ADR-003](../adr/ADR-003-meta-node.md): Meta Node for the Top-Level Record
