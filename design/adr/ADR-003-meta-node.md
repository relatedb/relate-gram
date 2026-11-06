---
id: ADR-003
title: Meta Node for the Top-Level Record
status: proposed
rfcs: [RFC-001]
created: 2026-11-06
---

# ADR-003: Meta Node for the Top-Level Record

## 1. Context

A gram file may open with a record, a set of key-value properties written in braces before any node or relationship:

```
{
  title: "Team roster",
  revision: 4
}

(a:Person {name: "Ada"})
```

The record describes the file as a whole. A property graph has no element for that: nodes and relationships hold data about things in the graph, and the database holds nothing about a graph's own title, source or revision. A loader that skips the record loses information, and an exporter has no way to write it back. The record has to be stored in the database as something, and whatever it is stored as has to be told apart from data, since a query such as `MATCH (n) RETURN n` returns every node the database holds.

Several forces shape the choice:

- Cypher offers labels and properties as the only ways to mark or find a node, and a label is the one that can be indexed and filtered cheaply. Relate's public library does not allow hiding an element from queries, so whatever the loader writes is visible to every query.
- A user can create nodes with any label, in Cypher or in a gram file. A marker that a user's data can also carry would let data masquerade as the record, or the record be mistaken for data.
- Property keys starting with `@` are rejected on load (ADR-001), reserving them for identity and metadata that later decisions define. The record's own properties are ordinary user keys.
- Loads are independent: a load never matches elements already in the database (ADR-001). A database can therefore hold the records of several loads.
- The grammar allows a record's values to be maps and arrays, but only values that become Cypher property values can be stored (RFC-002).

The parent RFC (RFC-001) says the record is kept as a "meta node" of a reserved kind, written back as the top-level record on export, and leaves the label, property layout and handling in queries to a decision (its question 7.1). This ADR makes that decision for the basic phase. It leaves open how a named graph's identity attaches to the meta node.

## 2. Decision

### 2.1 One meta node per record

A load whose file opens with a non-empty record creates exactly one meta node. A file with no record, or with an empty record `{}`, creates none. The meta node is a node like any other in storage, with no relationships to any other element.

### 2.2 The reserved label

The meta node carries the single label `_GramMeta`. Node labels beginning with `_Gram` are reserved: a gram file that gives any node such a label is a load error that reports the label and its position. The reservation covers all labels with the prefix, so that later decisions can add related labels without reclassifying existing files.

### 2.3 Properties

The record's properties become properties of the meta node, as written, under the same value mapping as any other property (RFC-002). A nested map or other value with no Cypher form is a load error. Record keys beginning with `@` are rejected on load as for any record (ADR-001), and are reserved for metadata about the file or its properties.

### 2.4 Identity and merging

The record has no identifier, so it is never merged. Two loads that both carry a record create two meta nodes. Property conflicts and `on-conflict` (ADR-001) do not apply to the record, because it appears once in a file.

### 2.5 Visibility in queries

The meta node is an ordinary node and appears in unfiltered queries, in counts, and in any export that lists nodes by Cypher. Queries that want data nodes only exclude it, for example with `WHERE NOT n:_GramMeta`. The loader does not hide it.

### 2.6 Export

The export reads the meta node separately from data nodes: nodes labelled `_GramMeta` are not written as node lines, take no identifier, and are excluded from the uniqueness checks of ADR-002.

- No meta node: no top-level record is written.
- One meta node: it is written first in the file, as the top-level record, followed by a blank line.
- More than one: the export fails, before any output, with an error that lists the meta nodes, until named graphs give each one its own graph to be exported with.
- A meta node that has labels other than `_GramMeta`, or that has relationships, is an export error listing the offenders, since a record cannot carry either.

The record is written one property per line, indented by two spaces, with keys sorted by Unicode code point and values spelled as the value-mapping decision determines:

```
{
  revision: 4,
  title: "Team roster"
}
```

A meta node with no properties is written as `{}`.

## 3. Alternatives Considered

3.1. **Discard the record.** Rejected: information in the source file is lost without notice, which contradicts the strict-subset rule that nothing is dropped.

3.2. **Copy the record's properties onto every node.** Rejected: it multiplies the data by the node count, the copies can diverge after edits, and a graph with no nodes cannot hold a record.

3.3. **Store the record as one JSON or text property on a node.** Rejected: its fields could not be queried or edited as properties.

3.4. **Mark the meta node with a property such as `@meta: true` and no label.** Rejected: `@` keys are reserved for other uses, and a property marker cannot be selected as cheaply as a label.

3.5. **An unreserved label such as `Meta` or `Gram`.** Rejected: users have such labels in their own data, so the record could be mistaken for data and the reverse.

3.6. **A label starting with `@`, to echo the reserved property prefix.** Rejected: a Cypher query must backtick-quote it every time (`` `@meta` ``), which makes the common filter awkward to write.

3.7. **Reserve only the exact label `_GramMeta`.** Rejected: later work on named graphs may add related labels, and reserving the prefix avoids reclassifying existing files.

3.8. **One meta node per database, merged across loads.** Rejected: a second load would silently mix or overwrite the first record's properties, and loads do not match existing content (ADR-001).

3.9. **Hide the meta node from queries.** Rejected: it is not possible through Relate's public interface, and the constitution requires depending on that interface only.

3.10. **Create a meta node for an empty record.** Rejected: there is nothing to preserve, and every file that begins `{}` would add an isolated node to the database.

## 4. Consequences

- The meta node is visible in `MATCH (n)` queries and in node counts. Users and tools must exclude `_GramMeta` where they want data only; this has to be documented.
- A database loaded from several files holds several meta nodes and cannot be exported as one file until named graphs exist, which then give each meta node its graph. Users who load several records into one database before then can still query and edit them.
- A file that opens with an empty record loses that record on a round trip. Its content was empty, so nothing is lost.
- A user who creates a `_GramMeta` node by hand through Cypher gets it exported as the record, and gets an export error if it has other labels or relationships.
- Labels beginning with `_Gram` cannot be used in gram files, even though gram permits them.
- The record's own value restrictions come from RFC-002. A record that holds a nested map fails to load until that decision allows otherwise.
- The advanced property metadata of RFC-002 (for example, a note that a property holds markdown) has a candidate home in `@`-prefixed keys on the meta node. How it is stored is left to that work.
- RFC-001 question 7.1 asks whether the meta node can carry the file's named graph. This ADR leaves it open; the named-graph decision attaches the graph's identity to the meta node.
- Follow-on decisions: the named-graph identity on the meta node, and the CLI flags for loading and exporting records, specified with the CLI integration.

## 5. Related

- RFCs: RFC-001
