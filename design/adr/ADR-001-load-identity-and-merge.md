---
id: ADR-001
title: Load Identity and Merge Semantics
status: proposed
rfcs: [RFC-001]
created: 2026-11-06
---

# ADR-001: Load Identity and Merge Semantics

## 1. Context

A gram file names an element by writing an identifier on it, and writes the same identifier again wherever the element is mentioned: in `(a:Person {name: "Ada"})-[:KNOWS]->(b)` and later `(b)-[:KNOWS]->(c)`, both `b` are one node. A loader therefore meets the same element several times, each time possibly carrying more labels or properties than before, and must decide what the combination means. Three things make the decision necessary before any loader is written:

- Gram identifiers are file-scoped. Relate's own element identity is internal and cannot be set by the loader, so a gram identifier is a name that exists only while the file is being read.
- A repeated element can disagree with itself: `(a {age: 30})` and later `(a {age: 31})`. Cypher's `MERGE … SET` resolves such a clash by overwriting, which is the opposite of what a loader for interchange files should do.
- Gram allows identifiers on relationships (`(a)-[r:KNOWS]->(b)`) and on nodes in the same file, and the grammar gives both the same kind of identifier. Nothing in the syntax stops one identifier from being used as a node and as a relationship.

In the tree-sitter grammar for gram, an identifier is a symbol, a backtick-quoted name, or an integer. A plain symbol cannot start with `@`. A property key may be a symbol, a backtick-quoted name or a double-quoted name, so a key beginning with `@` has to be quoted (`{"@k1": 1}` appears in the grammar's own test corpus).

The parent RFC (RFC-001) sets the direction: identifiers resolve references on load and are then dropped, the first appearance fixes whether an identifier names a node or a relationship, and a repeated appearance merges by default. It leaves the merge rules, relationship identity, anonymous elements and the path case to a decision (its open questions 7.3 to 7.6). This ADR makes those decisions for a single file loaded in the basic phase, in a form that cross-file merging can reuse later. It resolves questions 7.3, 7.4 and 7.6, and the default part of 7.5.

## 2. Decision

### 2.1 Identifier space and equality

A file has one identifier space shared by nodes and relationships. Two identifiers are equal if their text is equal after any backtick quoting is removed, so `` `a` `` and `a` name the same element, and the integer `42` and `` `42` `` do too. Identifiers are case-sensitive.

An element written without an identifier is anonymous (2.5).

### 2.2 Kind of an identifier

The first appearance of an identifier fixes its kind: a node if it appears in parentheses (as a node or a path endpoint), a relationship if it appears in brackets. Any later appearance as the other kind is a load error that reports both positions.

### 2.3 Merging repeated nodes

Appearances of a node with the same identifier are combined in file order:

- **Labels** are unioned. The merged node keeps labels in the order they first appear.
- **Properties** are unioned by key. If a key appears again with an equal value, nothing changes. Two values are equal only if they have the same Cypher type and the same value, so `1` and `1.0` differ, and two lists are equal if they have the same elements in the same order. Equality is judged after the value mapping (RFC-002), so `0x1F` and `31` are equal. A NaN never equals itself, so two NaN values conflict. Duplicate keys within a single record, such as `{a: 1, a: 2}`, are handled by the value-mapping decision (RFC-002 question 7.10).
- **A key with differing values** is a load error that reports the key, both values and both positions. A load option `on-conflict` can change this to `first` (keep the earlier value) or `last` (keep the later one). The default is `error`. With `first` or `last`, each discarded value is reported as a warning that names the key, the discarded value and both positions.
- A **bare reference**, such as `(a)` with no labels or properties, adds nothing to an existing node. If it is the first appearance, it creates a node with no labels and no properties, which later appearances fill in.

### 2.4 Merging repeated relationships

A relationship with an identifier has a fixed type, a fixed start node and a fixed end node, all set at its first appearance. A repeated appearance combines with it as follows:

- Its **type** MUST match, or the load fails with an error that reports both positions. An untyped relationship is governed by RFC-001 question 7.8.
- Its **endpoints** MUST name the same two nodes as the first appearance, by identifier, in the same direction; the written direction (`-[r]->` or `<-[r]-`) is irrelevant as long as start and end are the same. An anonymous endpoint in a repeat never matches and is an error.
- Its **properties** merge by the rules in 2.3, including `on-conflict`.

A relationship without an identifier is anonymous and never merges (2.5).

### 2.5 Anonymous elements

Each appearance of an element with no identifier creates a new element, whatever its labels and properties. `(a)-->(b)` written twice, with no identifier on the relationship, produces two relationships. A way to merge such relationships (for example by an explicit merge key) is not part of the basic phase.

### 2.6 Paths

A path is read as the chain of nodes and relationships it spells out, in left-to-right order, and each element follows the rules above as an ordinary appearance. An inner node (the `b` in `(a)-->(b)-->(c)`) is a node appearance like any other. The same identifier appearing twice in one path, as in `(a)-->(b)-->(a)`, names one node. Using an identifier as a node in one position and as a relationship in another, within a path or across paths, is the error in 2.2.

### 2.7 Scope of resolution and atomicity

Identifiers resolve only within the file being loaded. A load never matches an element already present in the database. A load either completes or leaves the database unchanged: all errors found while resolving the file abort the load before anything is written.

### 2.8 The reserved `@` key prefix

A property key that starts with `@` is reserved for identity metadata. In the basic phase, a record containing such a key, on a node, a relationship or the top-level record (the properties record that may open a gram file), is a load error that reports the key and its position. A later decision assigns `@id` and any related keys their meaning. Because a plain symbol cannot start with `@`, such a key can only appear in a gram file in quoted form.

### 2.9 Determinism

Elements are created in order of first appearance, and merges are applied in file order, so a given file and option set always produce the same graph.

## 3. Alternatives Considered

3.1. **Overwrite on conflict, like Cypher `MERGE ... SET`.** Rejected as the default: it discards a value, which makes the database differ from its source. It remains available as the opt-in `on-conflict: last`, with each discarded value reported.

3.2. **Keep the first appearance's properties and ignore later ones.** Rejected as the default for the same reason. Available as the opt-in `on-conflict: first`, with each discarded value reported.

3.3. **Accumulate conflicting values into a list.** Rejected: it changes the property's type from what any appearance wrote, and breaks type fidelity on export.

3.4. **Require every appearance of an element to be identical.** Rejected: gram files commonly declare a node once with its full record and mention it elsewhere by identifier alone, so the rule would reject ordinary files. Merging bare references is the common case.

3.5. **Merge relationships by (start, type, end), as Cypher `MERGE` does.** Rejected for the basic phase: it would merge distinct parallel relationships that a file writes deliberately, such as two `KNOWS` edges between the same pair with different properties. An explicit identifier already provides identity. A merge key can be added later as an opt-in.

3.6. **Separate identifier spaces for nodes and relationships.** Rejected: the same identifier could then denote two elements in one file, which makes a bare identifier ambiguous when it appears as a reference.

3.7. **Case-insensitive identifiers.** Rejected: gram identifiers are case-sensitive, and folding case would merge elements that a file writes as distinct.

3.8. **Treat `a` and `` `a` `` as different identifiers.** Rejected: the quoting only exists to permit characters a symbol cannot contain, so it carries no extra meaning.

3.9. **Treat an anonymous endpoint in a repeated relationship as unspecified.** Rejected: `()-[r]->()` could then silently attach to different nodes, and an error costs the user one explicit identifier.

3.10. **Reserve only the exact key `@id`.** Rejected: later work on namespaces may need further `@`-prefixed keys, and reserving the prefix now avoids reclassifying existing files later.

3.11. **Partial load, committing everything up to the first error.** Rejected: a half-loaded graph cannot be told apart from a complete one, and which half is loaded depends on file order.

## 4. Consequences

- A file that repeats a property with different values fails to load unless the user chooses `first` or `last`. Users must resolve genuine conflicts in the file, which is the intended effect.
- Loading the same file twice into one database produces two copies of every element, because resolution never matches existing elements. Loading into a database that already holds data, and the namespaces (named graphs inside one database, planned in the parent RFC) that make matching possible, are left to later work (RFC-001 open questions 7.9 and 7.2).
- Anonymous relationships never merge. Users who want `(a)-[:R]->(b)` written twice to be one relationship must give it an identifier. An opt-in merge key remains an open follow-up (RFC-001 question 7.5).
- User property keys starting with `@` cannot be loaded in the basic phase, even though gram permits them in quoted form.
- The rules are written for one file, but they depend only on the identity of an element and the order of its appearances, so the cross-file merging that RFC-001 plans can apply the same rules to elements matched by namespaced identity.
- Atomic load requires the loader to hold the resolved graph before writing it, so very large files need that much memory.
- Under `first` and `last` the loader reports every discarded value, so a load can emit many warnings.
- The `on-conflict` option needs a library setting and a CLI flag, specified with the CLI integration.
- RFC-001 should be updated to mark questions 7.3, 7.4 and 7.6 resolved and 7.5 partly resolved.
- Follow-on decisions: the export identity scheme, the meta node format (which also needs the top-level record's own rules for `@` keys), and the `@id` format.

## 5. Related

- RFCs: RFC-001
