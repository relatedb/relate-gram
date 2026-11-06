---
id: RFC-001
title: Gram Graph Support for Relate
status: draft
created: 2026-11-06
adrs: []
---

# RFC-001: Gram Graph Support for Relate

## 1. Summary

Gram is a plain-text notation for graphs and the patterns built from them. A gram file writes nodes as `(a:Person {name: "Ada"})`, relationships as `(a)-[:KNOWS]->(b)`, and chains of both as paths, so a small property graph can be read, diffed and edited by hand. Relate is an embeddable, serverless graph database queried with Cypher. It can render query results as gram text, but it cannot load a gram file, and it has no way to write a database back out as one.

This RFC proposes `relate-gram`, an extension to Relate that adds gram as a two-way format for graph data: load a gram file into an in-memory Relate database, and export a database to a gram file. The organizing claim is that the supported notation should be a strict subset of gram, namely the part that maps directly onto Cypher's property graph model: nodes, relationships, paths and the file's top-level record. Annotations and pattern notation, which have no Cypher counterpart, are out of scope.

The hard part of the work is element identity. Gram writes an identifier on every element so that other parts of the file can refer to it; a property graph database assigns its own internal identifiers and gives users no stable handle on them. The RFC sets out the intent for how identity is handled on load and on export, how several graphs can share one database in separate namespaces, and how the top-level record is kept. It records the direction and defers each detail to later RFCs and ADRs.

## 2. Problem / Motivation

**Graphs move between tools as scripts or machine formats, and neither can be read or reviewed as a graph.** Relate's current paths for getting graph data in and out are:

- `LOAD CSV`, which brings in rows, so every relationship has to be rebuilt by a Cypher statement that matches endpoints by some key column.
- JSON and Parquet snapshots, which are exact and complete but are machine formats for a whole database, not something to hand-edit or review in a diff.
- `--mode cypher`, which renders result values as literals. For `RETURN p.name AS name, p.age AS age` it prints `CREATE ({name: 'Alice', age: 30})`, one statement per row, with no relationship wiring.
- The WebAssembly build's `renderResult`, which renders a result that holds nodes as gram text. This direction exists, but only for display; nothing reads gram back in.

A gram file is a single artifact that holds nodes, relationships and their properties together, in a form people can diff and review. Without loading and exporting it, such a file cannot become a Relate database, and a Relate database cannot be handed to anyone as one.

**Identity gets in the way of round trips.** A gram file refers to elements by identifier, which is how `(a)-->(b)` and a later `(b)-->(c)` mean the same `b`. Relate's element identifiers are internal. A loader has to decide what a gram identifier means once the data is in the database, and an exporter has to invent identifiers when none are available. If these choices are left implicit, two loads of the same file can disagree, a second load can silently duplicate the first, and an export cannot be loaded back into an equivalent graph.

**A database holds one undifferentiated graph.** Interchange files tend to arrive in sets: several datasets, or revisions of the same dataset. Loading them into one database today means they merge into a single graph with nothing recording which element came from which source, and nothing to say when two files describe the same thing.

## 3. Goals

3.1. Load a gram file into an in-memory Relate database.

3.2. Export a Relate database to a gram file.

3.3. Support only the gram constructs that map directly onto Cypher's property graph model (nodes, relationships, paths and the top-level record), and reject everything else with an error that locates the offending text.

3.4. Make identity behavior explicit and configurable, with defaults that need no configuration:
  - On load, gram identifiers resolve references within the file and are then dropped.
  - On export, identifiers are generated, or copied from a property whose values are unique.

3.5. Preserve a gram file's top-level record by loading it as a distinguished "meta node".

3.6. Allow several graphs to live in one database in distinct namespaces. An element's namespaced identity is kept in a reserved `@id` property that survives a round trip through gram by becoming the element's identifier.

3.7. Load multiple graphs in managed fashion, each into its own namespace, and merge graphs loaded into the same namespace.

3.8. Integrate minimally with the `relate` command-line tool at first, and leave room for deeper CLI integration, identity-aware language bindings and graph-shaped client result structures.

## 4. Non-Goals

4.1. Gram annotations and pattern notation (subject patterns, nested and recursive structure). Neither has a counterpart in Cypher's data model.

4.2. Gram values that Cypher cannot store: nested maps and values with no Cypher type, such as tagged strings, ranges and measurements.

4.3. Gram as a durable storage format for Relate. The extension is an import and export format on top of the engine's own storage.

4.4. Query-language support. Queries remain Cypher.

4.5. Changing the Relate engine. The extension works through Relate's public interfaces.

4.6. Language bindings for TypeScript and Python, and client-side result structures, in the first phase. They are listed as goals for the longer term and are specified later.

## 5. Proposed Approach (high-level)

### 5.1 A strict subset defined by Cypher

The boundary of what is supported is set by one rule: a gram construct is accepted if and only if it maps directly onto Cypher's property graph model. Nodes carry zero or more labels and a property map. A relationship connects two nodes, carries exactly one type and a property map, and has a direction. A path is shorthand for a chain of nodes and relationships. Property values are limited to what Cypher can store: strings, integers, floats, booleans and homogeneous lists of those scalars.

Anything outside this rule is a load error, reported with its position in the file. Silently dropping or approximating a construct would make a loaded database differ from its source without notice.

The parser is expected to come from the existing gram tooling rather than being written afresh, and it will have to track the current gram grammar, which has gained support for scalar arrays inside property maps.

### 5.2 Identity on load

Gram identifiers exist to say "this element, again." On load they are used for exactly that, and then dropped:

- The first appearance of an identifier fixes whether it names a node or a relationship. Using the same identifier later as the other kind is an error.
- A later appearance of an identifier refers to the same element. By default the appearance merges into the element already loaded. The merge rules (what happens to differing property values, additional labels) are left to a later decision.
- After loading, the identifier is not stored on the element. The database's own internal identity applies.

Loading the same file twice into a fresh database each time therefore yields equivalent graphs, and a file that mentions an element in several places yields one element.

### 5.3 Identity on export

An exported file needs an identifier on each element that is referenced more than once. Export offers a small set of strategies:

- Use an element's namespaced `@id` value, when it has one (see 5.5).
- Copy the value of a configured property, when every element of that kind has a unique value for it. If some elements lack the property or the values collide, export fails and lists the offending elements.
- Otherwise generate identifiers (`n0`, `n1`, … for nodes; `r0`, `r1`, … for relationships), numbered in a deterministic order so that exports of the same database are stable and diffable.

The default behavior chooses among these in the order listed. Identifiers that are not plain gram symbols are quoted.

### 5.4 The top-level record and the meta node

A gram file may open with a record of properties describing the file as a whole. A property graph has no place for data about the graph itself, so the extension loads the record as a "meta node": a node of a reserved kind that is distinguishable from data nodes, holds the record's properties, and is written back as the top-level record on export. The reserved label, property layout and rules for excluding the meta node from ordinary queries are left to a later decision.

### 5.5 Namespaces and named graphs

A single Relate database can hold several graphs, each in a namespace. An element's namespaced identity is stored in a reserved `@id` property. On export, that value becomes the element's gram identifier, so identity survives the round trip: a graph exported from one database and loaded into another into the same namespace lines up with what is already there.

Two operations build on this:

- *Managed load of several graphs.* Each file loads into its own namespace, and the namespace is a first-class thing that can be listed, exported on its own, or removed.
- *Merging.* Loading a graph into a namespace that already exists merges elements whose `@id` values match, using the same merge defaults as in-file repetition (5.2).

The spelling of `@id` values, how a namespace is named, and how both appear in gram are left to a later decision.

### 5.6 Integration with Relate

The extension is a Rust library crate that depends on Relate's public library interface. Its first integration with the `relate` command-line tool is minimal: a way to run load and export. It is developed as a standalone project and may later be brought into the Relate project itself.

Identity-aware libraries for TypeScript and Python, and client-side data structures that carry element identity for tabular and graph-shaped results, are part of the intended direction. Their design is deferred until the core loader, exporter and CLI integration exist.

### 5.7 Phasing

Basic capability comes first: load and export of the strict subset, with the default identity behavior. Namespaces, managed multi-graph loading and merging come second, since they depend on the identity model being settled. Language bindings and client structures come last.

## 6. Alternatives Considered

6.1. **Keep using Cypher scripts as the interchange format.** Rejected: a script describes how to build a graph rather than the graph, relationships must be rebuilt by matching nodes on some key, and nothing in a script says that two statements mention the same element.

6.2. **Use the existing JSON or Parquet snapshots.** Rejected for interchange: they are complete and exact for a whole database, but are machine formats that cannot be reviewed or hand-edited as a graph, and do not support loading several graphs into separate namespaces.

6.3. **Support the full gram language.** Rejected: annotations and pattern notation have no meaning in Cypher's data model. Supporting them would mean inventing a mapping, and files that use them would not round-trip.

6.4. **Store gram identifiers on elements as an ordinary property by default.** Rejected as the default: identifiers in a file are scoped to the file, and persisting them would make a second load duplicate the first. The reserved `@id` property provides persisted identity as an explicit choice in namespaces.

6.5. **Make gram a storage format (a snapshot codec) for Relate.** Rejected for now: the strict subset cannot carry everything a database holds, such as indexes, schema and vector types, so it does not suit a durable form. It remains possible to revisit if gram's coverage grows.

6.6. **Contribute directly to Relate rather than building a standalone extension.** Deferred: a standalone project can move faster while the identity model is settled, and the code can be brought into Relate afterward.

## 7. Open Questions

7.1. What is the meta node's format: its label, how its properties are stored, how it is kept apart from data nodes, and whether it can carry the file's namespace? (A candidate ADR.)

7.2. What is the format of an `@id` value, and how is a namespace named and written in gram? Is namespacing a single prefix on an identifier, a separate property on each element, or something else?

7.3. What are the default merge rules when an element appears more than once, whether within one file or across files in the same namespace? In particular, what happens to conflicting property values, and to labels that differ between appearances?

7.4. What happens to relationships when a merge touches their endpoints? Is a relationship's identity independent of its endpoints, or can an identical `(a)-[:R]->(b)` appearing twice be one relationship or two?

7.5. How does a path that reuses an identifier on one of its inner nodes interact with the rule that the first appearance fixes an identifier as a node or relationship?

7.6. What is the exact subset of property values to support, and how are lists, numeric widths and string escaping mapped between gram and Cypher?

7.7. How much of a gram file's text survives a load and export, beyond its graph? Do comments, element ordering and layout matter?

7.8. Which parts of the existing gram parser need updating to match the current gram grammar, and should that work happen before or alongside the loader?

7.9. What does the minimal `relate` CLI integration look like (verbs, flags, defaults), and what does it need from Relate's extension mechanism?

7.10. What are the follow-on RFCs? Candidates are identity and namespaces, the meta node, CLI integration, and language bindings with client-side result structures.

## 8. ADRs

_(populated automatically as `/adr-create` links ADRs to this RFC)_
