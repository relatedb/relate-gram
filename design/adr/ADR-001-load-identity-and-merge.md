---
id: ADR-001
title: Load Identity and Merge Semantics
status: accepted
rfcs: [RFC-001, RFC-003]
created: 2026-11-06
---

# ADR-001: Load Identity and Merge Semantics

> The amendments of 2026-11-08 (identifiers kept as `@id`, matching against the database, identity from a property) took effect on 2026-11-08, when ADR-004 (which amends Constitution III) was accepted.
>
> Amended 2026-11-08 after SPIKE-001: §2.7 and §2.10 now say how the loader matches against the database, and alternatives 3.19 and 3.20 record the options the spike rejected.

## 1. Context

A gram file names an element by writing an identifier on it, and writes the same identifier again wherever the element is mentioned: in `(a:Person {name: "Ada"})-[:KNOWS]->(b)` and later `(b)-[:KNOWS]->(c)`, both `b` are one node. A loader therefore meets the same element several times, and must decide which appearances may say what the element is and what a later one means. Three things make the decision necessary before any loader is written:

- Gram identifiers are file-scoped. Relate's own element identity is internal and cannot be set by the loader, so a gram identifier has no home in the database unless the loader stores it as a property of the element.
- A repeated element can disagree with itself: `(a {age: 30})` and later `(a {age: 31})`. Cypher's `MERGE … SET` resolves such a clash by overwriting, which is the opposite of what a loader for interchange files should do. Allowing an element to be defined at only one site removes the clash instead of resolving it.
- Gram allows identifiers on relationships (`(a)-[r:KNOWS]->(b)`) and on nodes in the same file, and the grammar gives both the same kind of identifier. Nothing in the syntax stops one identifier from being used as a node and as a relationship.

In the tree-sitter grammar for gram, an identifier is a symbol, a backtick-quoted name, or an integer. A plain symbol cannot start with `@`. A property key may be a symbol, a backtick-quoted name or a double-quoted name, so a key beginning with `@` has to be quoted (`{"@k1": 1}` appears in the grammar's own test corpus).

The parent RFC (RFC-001) sets the direction: identifiers resolve references on load and are kept as the element's `@id`, the first appearance fixes whether an identifier names a node or a relationship, and each element is defined at one site and referred to elsewhere. It leaves the merge rules, relationship identity, anonymous elements and the path case to a decision (its open questions 7.3 to 7.6). This ADR makes those decisions for a file loaded into a single graph space (the first phase of RFC-003), in a form that named graphs can reuse later. It resolves questions 7.3, 7.4 and 7.6, and the default part of 7.5.

## 2. Decision

### 2.1 Identifier space and equality

A file has one identifier space shared by nodes and relationships. Two identifiers are equal if their text is equal after any backtick quoting is removed, so `` `a` `` and `a` name the same element, and the integer `42` and `` `42` `` do too. Identifiers are case-sensitive.

An element written without an identifier is anonymous (2.5).

An element's identifier is stored on it as the string property `@id` (2.7). The text is what is stored, so the integer `42` is stored as `"42"`. Identity from a configured property (2.10) is the exception: no `@id` is written for that kind.

### 2.2 Kind of an identifier

The first appearance of an identifier fixes its kind: a node if it appears in parentheses (as a node or a path endpoint), a relationship if it appears in brackets. Any later appearance as the other kind is a load error that reports both positions.

### 2.3 Defining and referencing nodes

A node appearance that carries labels or a record (properties between braces, `{}` included) is a *definition*. A node appearance with neither is a *reference*. A node identifier has at most one definition in a file, and any number of references, before or after it.

- **Two definitions of one identifier** are a load error that reports both positions and says to define the element once and refer to it elsewhere as `(a)`. This holds when the two definitions are identical.
- The node's labels, in the order written, and its properties are exactly those of its definition. A reference adds nothing to it.
- **An identifier that is only referenced** creates a node with no labels and no properties, unless the database already holds an element with that `@id` (2.7). Because creating one is often a typo (`(alcie)` for `(alice)`), the load lists such identifiers in a warning. `(a {})` is the explicit definition of a node with no content, and it is not warned about.
- Duplicate keys within a single record, such as `{a: 1, a: 2}`, are a load error (ADR-006).

A file therefore never contradicts itself about an element, and a load has no conflict option.

### 2.4 Defining and referencing relationships

A relationship with an identifier has a fixed type, a fixed start node and a fixed end node. A relationship appearance that carries a type or a record is its definition, and an appearance with neither (`[r]`) is a reference. The same rules apply as for nodes:

- A relationship identifier has at most one definition, which carries the type and the properties. A second definition is a load error that reports both positions.
- Every appearance MUST name the same two nodes as the first appearance, by identifier, in the same direction; the written direction (`-[r]->` or `<-[r]-`) is irrelevant as long as start and end are the same. An anonymous endpoint in a later appearance never matches and is an error.
- A relationship identifier that is only referenced has no type, and a Cypher relationship has exactly one, so it is a load error. Untyped relationships and arrow forms are governed by ADR-008.

A relationship without an identifier is anonymous and never merges (2.5).

### 2.5 Anonymous elements

Each appearance of an element with no identifier creates a new element, whatever its labels and properties. `(a)-->(b)` written twice, with no identifier on the relationship, produces two relationships. A way to merge such relationships (for example by an explicit merge key) is not part of the first phase. No identifier is generated for an anonymous element in that phase: it is stored without `@id`, a later load never matches it, and loading the same file again can duplicate it.

### 2.6 Paths

A path is read as the chain of nodes and relationships it spells out, in left-to-right order, and each element follows the rules above as an ordinary appearance. An inner node (the `b` in `(a)-->(b)-->(c)`) is a node appearance like any other. The same identifier appearing twice in one path, as in `(a)-->(b)-->(a)`, names one node. Using an identifier as a node in one position and as a relationship in another, within a path or across paths, is the error in 2.2.

### 2.7 Matching elements already in the database

Within a file, identifiers resolve references. Each identified element is also stored with its identifier as the property `@id`, which the loader writes (2.8). A load matches the file's identified elements against the database by `@id`:

- An element whose `@id` is already present is the same element and is updated: its labels are unioned with the definition's, and each property the definition states takes the file's value. Properties the file does not state, and elements the file does not mention, are left alone (RFC-003).
- An element whose `@id` is absent is created.
- A reference whose identifier is already present binds to that element and changes nothing.
- An `@id` that belongs to the other kind of element (a node where the file says relationship, or the reverse) is a load error. So is a relationship whose type or endpoints differ from the stored ones; the error reports both.
- Elements in the database without an `@id` are never matched, unless a configured identity property matches them (2.10).
- If more than one element in the database has the `@id` that a file element matches, the load fails and lists them. A relationship's endpoints are compared with the stored ones by the elements they resolve to, not by identifier text.

A load either completes or leaves the database unchanged: all errors found while resolving the file, including those from matching against the database, abort the load before anything is written. Named graphs, in a later phase, scope the match to one graph (RFC-003).

**How the loader matches.** Relate can look up a node by property quickly only through an index, and an index needs a label, while a file may add a label to an element the database already holds (SPIKE-001). So the loader does not look elements up one at a time. Inside one transaction it first reads every existing element's internal id and `@id` in a single scan (nodes and relationships separately, relationships with their type and endpoint ids), builds a table from `@id` text to elements in the library, and resolves the whole file against that table. This is also where duplicate `@id` values in the database are found. The writes then address matched elements by their internal id, in batches. The loader creates no index or constraint.

**Transaction.** The scan and all writes run in one explicit transaction (`begin`, `commit`). Any error rolls it back. Relate leaves a transaction that has had a failed statement usable only for `rollback`, so the loader does nothing further after a failed statement except roll back.

### 2.8 The reserved `@` key prefix

A property key that starts with `@` is reserved for identity metadata. In the first phase, a record containing such a key, on a node, a relationship or the top-level record (the properties record that may open a gram file), is a load error that reports the key and its position. `@id` holds an element's identifier (2.7) and is written only by the loader; any other key starting with `@` stays reserved for later decisions. Because a plain symbol cannot start with `@`, such a key can only appear in a gram file in quoted form.

### 2.9 Determinism

Elements are created in order of first appearance, so a given file always produces the same graph.

### 2.10 Identity from a property

Many graphs already carry a natural identifier, such as an `email` or a `uuid`. A load can use it as the element's identity instead of the gram identifier. The settings are `node-id-property` and `relationship-id-property`, the same two that ADR-002 uses on export. Each names one property and applies to one kind of element, and by default neither is set, so identity is the gram identifier stored as `@id` (2.7).

When a setting is given for a kind:

- The property's value, taken as text (2.1), is the element's identity, and the key by which a load matches elements of that kind already in the database. Relate treats the integer `42` and the string `"42"` as different values and does not enforce uniqueness across them, so the loader reads every existing element that has the property in one scan (as in 2.7), compares the text itself, and reports two elements whose values are equal as text as duplicates. The gram identifier resolves references within the file only. It is not stored, and no `@id` is written for elements of that kind.
- A definition in the file that lacks the property is anonymous (2.5), and the load reports it. A node that is only referenced has no property either, so it is also anonymous and is reported with the warning of 2.3.
- The value MUST be a non-empty string or an integer; any other value is a load error that reports its position. Two definitions of one kind with the same value are a load error that reports both positions.
- Matching ignores `@id`. An element of that kind in the database with the same property value is the same element and is updated as in 2.7, whether or not it has an `@id`, since naming the key is the explicit act of adoption. If more than one existing element has the value, the load fails and lists them.
- The property remains an ordinary property of the element, written from the file's definition like any other.

A graph exported with a property setting reloads by matching that property when the same setting is given to the load. With different settings in each direction the identities do not line up and the load duplicates elements.

## 3. Alternatives Considered

3.1. **Overwrite on conflict, like Cypher `MERGE ... SET`.** Rejected: it discards a value, which makes the database differ from its source. With one defining site per element, a file has no conflict to resolve.

3.2. **Keep the first appearance's properties and ignore later ones.** Rejected for the same reason.

3.3. **Accumulate conflicting values into a list.** Rejected: it changes the property's type from what any appearance wrote, and breaks type fidelity on export.

3.4. **Require every appearance of an element to be identical.** Rejected: gram files commonly declare a node once with its full record and mention it elsewhere by identifier alone, so the rule would reject ordinary files. References, which carry no content, are the common case and are always allowed.

3.5. **Merge relationships by (start, type, end), as Cypher `MERGE` does.** Rejected for the first phase: it would merge distinct parallel relationships that a file writes deliberately, such as two `KNOWS` edges between the same pair with different properties. An explicit identifier already provides identity. A merge key can be added later as an opt-in.

3.6. **Separate identifier spaces for nodes and relationships.** Rejected: the same identifier could then denote two elements in one file, which makes a bare identifier ambiguous when it appears as a reference.

3.7. **Case-insensitive identifiers.** Rejected: gram identifiers are case-sensitive, and folding case would merge elements that a file writes as distinct.

3.8. **Treat `a` and `` `a` `` as different identifiers.** Rejected: the quoting only exists to permit characters a symbol cannot contain, so it carries no extra meaning.

3.9. **Treat an anonymous endpoint in a repeated relationship as unspecified.** Rejected: `()-[r]->()` could then silently attach to different nodes, and an error costs the user one explicit identifier.

3.10. **Reserve only the exact key `@id`.** Rejected: later work on named graphs may need further `@`-prefixed keys, and reserving the prefix now avoids reclassifying existing files later.

3.11. **Partial load, committing everything up to the first error.** Rejected: a half-loaded graph cannot be told apart from a complete one, and which half is loaded depends on file order.

3.12. **Union labels and properties across appearances, with an error on differing values and an `on-conflict` option (the earlier form of this decision).** Rejected: an element's content could be spread over several places, so a reader has to find all of them to know what the element is, and the option needs `first` and `last` modes whose discarded values must be reported.

3.13. **Allow repeated definitions when they are identical.** Rejected: it adds an equality rule (type, value mapping, NaN) to save a one-line edit, and a file with two identical definitions still has two places to maintain.

3.14. **Make an identifier that is only referenced an error.** Rejected: a node with no content that others point to is legitimate in a property graph, and the rule would force `(a {})` onto every such node. A warning that lists them catches typos instead.

3.15. **Drop identifiers after a load (the earlier form of this decision).** Rejected: identity is fundamental to gram, loading a file again would duplicate every element, and an export could not write the identifiers back.

3.16. **Generate an identifier for each anonymous element in the first phase.** Deferred: with one graph space there is no scope to derive a collision-free identifier from, and a stored identifier is hard to retire. RFC-003 generates them in the second phase, scoped by graph name.

3.17. **Let the identity property replace `@id` for every kind at once.** Rejected: one setting per kind keeps a graph that mixes natural keys and file identifiers loadable, and a kind without a setting behaves as before.

3.18. **Guess the identity property from names such as `name`, `title` or `*id`.** Rejected: a wrong guess merges distinct elements without notice. Naming the property is explicit.

3.19. **Match each element with `MERGE (n:Label {`@id`: …})`.** Rejected: the lookup needs a label, so an existing element that lacks one of the file's labels is missed and duplicated, which defeats label union. A lookup without a label is a full scan per element: 2,000 of them over 50,000 nodes took 54 seconds (SPIKE-001).

3.20. **Create a uniqueness constraint on `@id` for each label.** Rejected: constraints are per label and cannot cover an element across labels, a constraint creates an index that a later `CREATE INDEX` cannot duplicate, and a constraint violation mid-load reports less than the loader's own duplicate listing.

## 4. Consequences

- A file that defines an element at more than one site fails to load, even when the definitions agree. Users must consolidate the content into one definition and refer to it elsewhere, which the error says.
- Loading the same file twice updates the elements that carry identifiers and duplicates the anonymous ones. A load into a database that already holds data matches by `@id`. Named graphs, which scope the match, are left to later work (RFC-003).
- Anonymous relationships never merge. Users who want `(a)-[:R]->(b)` written twice to be one relationship must give it an identifier. An opt-in merge key remains an open follow-up (RFC-001 question 7.5).
- User property keys starting with `@` cannot be loaded in the first phase, even though gram permits them in quoted form.
- The rules for a file depend only on the identity of an element and the order of its appearances, so named graphs can apply them to elements matched by identity within one graph.
- Storing identifiers extends what Constitution III allows, since it says identifiers "are not stored". ADR-004 amends Constitution III.
- In the first phase every file loads into one identifier space. Two unrelated files that define the same identifier, as ADR-002's generated identifiers `n0` and `r0` always do, are one element, and the later file's values win. The summary reports updated elements, so the overlap is visible. Named graphs scope identifiers per graph.
- Every identified element carries an `@id` property that `RETURN n` and `properties(n)` show. `SET n = {…}` removes it, and Cypher can create duplicate `@id` values, since constraints in Relate apply per label and cannot cover every element.
- Atomic load requires the loader to hold the resolved graph before writing it, so very large files need that much memory. The table of existing `@id` values costs memory proportional to the identified elements already in the database, and the scan reads all of them even for a small file; 50,000 nodes took 66 ms. Larger databases and persisted stores were not measured.
- The scan and the writes share one transaction, so the loader sees one state of the database. How Relate resolves a concurrent writer that changes the same elements between them was not tested.
- A load with an identity property can update elements that carry no `@id`, such as ones created through Cypher. Exporting and loading with different property settings duplicates elements.
- Follow-on decisions: generated identifiers and named graphs (RFC-003), and the summary a load prints, specified with the CLI integration.

## 5. Related

- RFCs: RFC-001
