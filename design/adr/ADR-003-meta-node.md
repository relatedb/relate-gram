---
id: ADR-003
title: Meta Nodes and the Meta Graph
status: proposed
rfcs: [RFC-001]
created: 2026-11-06
---

# ADR-003: Meta Nodes and the Meta Graph

## 1. Context

A gram file may open with a record, a set of key-value properties written in braces before any node or relationship:

```
{
  name: "roster",
  revision: 4
}

(a:Person {name: "Ada"})
```

The record describes the file as a whole. A property graph has no element for that: nodes and relationships hold data about things in the graph, and nothing holds data about the graph itself. A loader that skips the record loses information, and an exporter has no way to write it back. The record has to be stored in the database as something, and the database has to be able to tell it from data.

Several forces shape the choice:

- A node cannot be hidden. Relate's public library offers no way to exclude an element from queries, so whatever the loader writes is visible to every query that scans nodes.
- The project plans for one database to hold several graphs, each a named graph, with a graph's identity kept in a reserved `@id` property (RFC-001). A graph needs a name and a place to keep information about itself, which is what the record is.
- A gram file's top-level record also says what kind of file it is. In the wider gram ecosystem the `kind` property selects the semantics, conventions and style of the contents, for example `schema` for a file of constraints. Every gram file is valid data, but only one kind is a property graph.
- The ecosystem names files `graph-name.kind.gram`, so a file name can carry the graph's name and its kind.
- Property keys starting with `@` are rejected on load (ADR-001), so data loaded from a file never has an `@id`.
- The grammar allows a record's values to be maps and arrays, but only values that become Cypher property values can be stored (RFC-002).

The parent RFC (RFC-001) says the record is kept as a "meta node" and written back as the top-level record on export. It leaves the node's label, property layout and handling in queries to a decision (its question 7.1). This ADR decides them for a design in which meta nodes belong to a reserved graph of their own. How elements are recorded as members of any named graph, and the format of `@id`, remain for the named-graph decision (RFC-001 question 7.2).

## 2. Decision

### 2.1 One supported kind

The loader and exporter support one kind of gram file, `property-graph`. This is an assumption of the project, not a property of gram: other kinds exist in the ecosystem and are not loaded.

- A file whose name follows `graph-name.kind.gram` and whose kind is not `property-graph` is a load error.
- A top-level record with a `kind` property whose value is not the string `property-graph` is a load error that reports the position.
- The error message states the assumption: "relate-gram loads only kind 'property-graph'; this file is kind '<kind>'".
- A file with no kind, in its name or its record, is taken to be `property-graph`.

The `kind` property is checked and then dropped: it is not stored on the meta node, and exports do not write it. The supported kind is a single constant.

### 2.2 The name of a graph

A load resolves a graph name from, in order of precedence:

1. an explicit name given by the caller (a library setting, with a CLI flag specified later);
2. the record's `name` property, which MUST be a non-empty string;
3. the file name: the part before `.gram`, and if that part contains a dot, the part before its last dot, with the last segment read as the kind (2.1). A name with a dot in it therefore cannot come from a file name and must be given by the record or the caller.

When the caller's name and the record's `name` differ, the caller's wins and the load reports a warning.

A file that has a non-empty record and no resolvable name is a load error that asks for a name. A file with no record and no resolvable name, such as one read from standard input, is loaded as an unnamed graph and gets no meta node.

### 2.3 The meta graph and the meta node

Meta nodes are the members of one reserved named graph, called `@@`. The name falls under the reserved `@` prefix, so no user graph can have it. `@@` has no form in gram: its only counterpart in a file is the top-level record, so a file never refers to it.

A named graph has exactly one meta node, written:

```
(:Graph {@id: "roster", @hash: "sha256:9f2c…", revision: 4})
```

- The label is `Graph`.
- `@id` holds the graph's name.
- `@hash` holds the SHA-256 digest of the file the graph was loaded from (2.4), written `sha256:` followed by 64 lowercase hexadecimal digits.
- The record's other properties are stored as ordinary properties, under the value mapping of RFC-002. `name` is not stored separately, since `@id` holds it, and `kind` is dropped (2.1). Record keys starting with `@` remain load errors (ADR-001).
- The meta node has no relationships.

Until the named-graph decision defines how an element is recorded as a member of a graph, a node is recognized as a meta node if it has the label `Graph` and an `@id` property. A node that has the label `Graph` and no `@id`, such as one loaded from a file, is ordinary data. The named-graph decision replaces this rule with membership of `@@`.

A file that has a graph name creates a meta node even when it has no record, so that the graph is named in the database.

### 2.4 Repeated loads of a named graph

A load computes the SHA-256 digest of the file's raw bytes, exactly as read, before any decoding or parsing. It stores the digest in the meta node's `@hash`. The same digest is computed for input read from standard input.

When a load resolves a name for which the database already holds a meta node with that `@id`, the digests are compared:

- **Equal:** the file is unchanged. The load writes nothing, succeeds, and reports that the graph is already loaded.
- **Different, or the stored meta node has no `@hash`:** the file has changed. Merging the new version into the existing graph is the intended behavior, and it needs the elements of the existing graph to be matched by identity, which they do not carry yet (see below). Until the named-graph decision supplies that, such a load fails with an error that names the graph and both digests and says what to do: load the file under a different graph name, or wait for merge support.

A gram identifier is dropped after a load (ADR-001), so nothing in the database says which existing element a repeated one corresponds to. Matching needs each element to keep its identity in `@id` and to record the graph it belongs to, which is the named-graph decision (RFC-001 question 7.2). That decision also defines how elements without an identifier are treated on a merge, since they never merge (ADR-001).

### 2.5 Visibility in queries

Meta nodes appear in every query that scans nodes, in counts and in exports by Cypher. Queries that want data nodes only exclude them, for example with ``WHERE NOT (n:Graph AND n.`@id` IS NOT NULL)``. The loader does not hide them.

### 2.6 Export

The exporter reads meta nodes separately from data nodes. A meta node is not a data element: it is not written as a node line, takes no gram identifier, and is not subject to the checks that ADR-002 applies to `@` keys on data elements. The record is built from `@id` (as `name`) and the ordinary properties; `@hash` describes the source file, not the graph, and is not written.

- No meta node: no top-level record is written.
- One meta node: it is written first in the file as the top-level record, followed by a blank line.
- More than one meta node: the export fails, before any output, with an error listing them, until data elements record the graph they belong to.
- A meta node that has labels other than `Graph` or has relationships is an export error listing the offenders.

The record has one property per line, indented by two spaces, with `name` taken from `@id` and keys sorted by Unicode code point, and values spelled as the value-mapping decision determines:

```
{
  name: "roster",
  revision: 4
}
```

The record always includes `name`, so an exported file names its graph whatever the original file name was.

## 3. Alternatives Considered

3.1. **Discard the record.** Rejected: information in the source file is lost without notice, which contradicts the strict-subset rule that nothing is dropped.

3.2. **Copy the record's properties onto every node.** Rejected: it multiplies the data by the node count, the copies can diverge after edits, and a graph with no nodes cannot hold a record.

3.3. **Store the record as one JSON or text property on a node.** Rejected: its fields could not be queried or edited as properties.

3.4. **Hide the meta node from queries.** Rejected: Relate's public interface cannot do it, and the constitution requires depending on that interface only. Placing meta nodes in their own graph makes the separation a matter of membership instead.

3.5. **A reserved label such as `_GramMeta` as the marker.** Rejected: once meta nodes belong to the meta graph, membership is the marker, and `Graph` says what the node is. The interim recognition rule (label plus `@id`) avoids misreading data nodes that have a `Graph` label.

3.6. **Name the meta graph `meta`.** Rejected: a user could name a graph `meta`. The `@` prefix is reserved already, so `@@` cannot collide.

3.7. **One meta node per database, merged across loads.** Rejected: a second load would mix or overwrite the first record's properties. One meta node per graph keeps each graph's information apart.

3.8. **Merge a changed file into the existing graph now.** Deferred, not rejected: it is the intended behavior, but identifiers are dropped on load, so existing elements cannot be matched to the new file's elements. Merging by name alone would duplicate every element.

3.9. **Fail on any repeated graph name, without a digest.** Rejected: an unchanged file would fail on reload, where a digest comparison makes it a harmless no-op.

3.10. **Hash the normalized content of the parsed graph.** Rejected: it needs a canonical form that does not depend on element identifiers, which the export does not guarantee (ADR-002). Raw bytes are exact and cheap, and a reformatted file is treated as changed.

3.11. **Require every graph to be named.** Rejected: loading a small fixture from standard input with no name would then fail for no benefit.

3.12. **Treat every dot-separated segment of a file name as a possible kind.** Rejected: graph names such as `v1.2` would be misread. Only the last segment is the kind.

3.13. **Accept `relate` as an alias for `property-graph`.** Rejected: a second name for one kind would have to be kept indefinitely. The kind is a single constant that can change in one place.

3.14. **Store `kind` on the meta node.** Rejected: only one kind is supported, so the value carries no information, and an exported file is the same kind whatever the original said.

## 4. Consequences

- The meta node is visible in `MATCH (n)` queries and counts. Users and tools exclude it by its `@id` and `Graph` label until named graphs give a membership filter; this has to be documented.
- The interim recognition rule must be replaced when named graphs exist. Once data elements also carry `@id`, a data node labelled `Graph` would match it.
- Loading the same named file twice is a no-op, and a changed file fails until named graphs allow a merge. This amends the consequence recorded in ADR-001, which expected the second load to duplicate every element. Unnamed loads still duplicate.
- A reformatted or re-commented file has a different digest and counts as changed.
- A database loaded from several differently named files holds several meta nodes and cannot be exported as one file until elements record the graph they belong to.
- A graph name with a dot can only come from the record or the caller. A file `roster.v2.gram` is read as graph `roster`, kind `v2`, and fails.
- Exports always write `name` and never `kind`, so a file that omitted `name` gains it on a round trip. A file that carried `kind: "property-graph"` loses it.
- A file read from standard input with no record and no name loads as an unnamed graph and leaves no trace of itself in the database.
- `Graph` is an ordinary label in gram files and is not reserved. A file cannot set `@id`, so it cannot forge a meta node.
- The `kind` assumption is recorded here; the file-kind conventions of the wider gram ecosystem (for example `.schema.gram`) are not loaded, and the error says so.
- Follow-on decisions: how elements record membership of a named graph and the format of `@id` (RFC-001 question 7.2), loading into an existing graph, and the CLI flag for the graph name, specified with the CLI integration.

## 5. Related

- RFCs: RFC-001
