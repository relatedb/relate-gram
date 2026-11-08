---
id: ADR-005
title: Graph Metadata and Meta Nodes
status: proposed
rfcs: [RFC-001, RFC-003]
created: 2026-11-08
---

# ADR-005: Graph Metadata and Meta Nodes

## 1. Context

A gram file may open with a record that describes the file as a whole, such as `{name: "roster", revision: 4}`. A property graph has no element for that, and a database that holds several graphs needs somewhere to keep what it knows about each one: its name, a digest of the file it was loaded from, and the record's properties. A named graph here is a set of elements that the database keeps apart from other graphs, each element recorded as a member (RFC-003).

Several forces shape where that metadata lives:

- A node cannot be hidden. Relate's public library offers no way to exclude an element from queries, so a node written for each graph appears in every query that scans nodes.
- Relate's ontology document has a `custom` extension, a free-form map for data that a tool keeps alongside a model, which the specification asks tools to group under a key named after themselves. It adds nothing to queries. It has costs: the store holds a single ontology document, `db.ontology.set` replaces all of it, the procedures run only in their own implicit transaction, and they are behind a preview opt-in (`RELATE_PREVIEW=ontology`) because the specification is not final (RFC-003 question 7.18).
- Metadata held only in the ontology document cannot be listed or queried with Cypher. Nodes can.
- The ecosystem names files `graph-name.kind.gram`, so a file name can carry a graph's name.
- Property keys starting with `@` are rejected on load (ADR-001), so a file cannot set `@id`. The loader writes it on the elements it stores and on a meta node.
- The record's values are checked under the value mapping of RFC-002.

RFC-003 stages the work in three phases. The first phase stores no metadata (ADR-003). This ADR covers the second and third phases. Several of its rules depend on open questions in RFC-003: the record key that names a graph (7.12), whether the file name is a fallback (7.13), which graph names are reserved (7.14), and the risks of using the ontology document (7.18). It stays proposed until those are settled.

## 2. Decision

### 2.1 The name of a graph

A load resolves a graph name from, in order of precedence:

1. an explicit name given by the caller (a library setting, with a CLI flag specified later);
2. the record's `name` property, which MUST be a non-empty string;
3. the file name: the part before `.gram`, and if that part contains a dot, the part before its last dot, with the last segment read as the kind (ADR-003). A name with a dot in it therefore cannot come from a file name and must be given by the record or the caller.

A resolved name equal to `@@` or `@_` is a load error, since those are the reserved graphs (2.4). Any other name beginning with `@` is also a load error until RFC-003 question 7.14 settles whether such names are allowed.

When the caller's name and the record's `name` differ, the caller's wins and the load reports a warning.

A file that has a non-empty record and no resolvable name is a load error that asks for a name. A file with no record and no resolvable name, such as one read from standard input, is loaded into the catch-all graph `@_` (RFC-003), which has no digest of its own.

### 2.2 Where the metadata is held

- **Second phase.** The metadata is held in the ontology document under `extensions.custom.relate-gram`, one entry for each graph, keyed by the graph's name. Named graphs are therefore behind Relate's ontology preview opt-in. An entry has this shape, with the graph's name as the map key only:

  ```json
  {"roster": {"hash": "sha256:9f2c…", "properties": {"revision": 4}}}
  ```

  `hash` is the digest (2.3, 2.5) and `properties` holds the record's other properties. The shape is provisional until a spike confirms that Relate's ontology procedures accept and return a `custom` entry unchanged; this ADR is not accepted before then.
- **Third phase.** Each entry is materialized as a meta node in a reserved graph, `@@` (2.4), so that graphs can be listed and queried with Cypher.

### 2.3 The metadata

For each named graph the loader keeps:

- its name;
- the SHA-256 digest of the file it was loaded from, written `sha256:` followed by 64 lowercase hexadecimal digits;
- the record's other properties, under the value mapping of RFC-002. `name` is not stored separately, and `kind` is dropped (ADR-003). Record keys starting with `@` are load errors (ADR-001).

A file that has a graph name creates an entry even when it has no record, so that the graph is named.

### 2.4 The meta graph and the meta node

Meta nodes are the members of one reserved named graph, called `@@`. `@@` has no form in gram: its only counterpart in a file is the top-level record, so a file never refers to it. A graph has exactly one meta node. For a graph named `roster`, for example, it is written:

```
(:Graph {@id: "roster", @hash: "sha256:9f2c…", revision: 4})
```

- The label is `Graph`, `@id` holds the graph's name, and `@hash` holds the digest.
- The record's other properties are ordinary properties.
- The meta node has no relationships.
- `@id` and `@hash` are written by the loader and never read from the file. A meta node holds the graph's name, not a gram element identifier.

Data elements carry `@id` too (ADR-001), so a meta node cannot be recognized by its label and `@id`: a data node labelled `Graph` would match. A meta node is recognized by membership of `@@` (RFC-003).

### 2.5 Repeated loads of a named graph

A load computes the SHA-256 digest of the file's raw bytes, exactly as read, before any decoding or parsing. The same digest is computed for input read from standard input. When the database already holds an entry for the resolved name, the digests are compared:

- **Equal:** the file is unchanged. The load writes nothing, succeeds, and reports that the graph is already loaded.
- **Different, or the stored digest is missing:** the file has changed. The load merges the new version into the existing graph, matching elements by their stored `@id` (RFC-003).

The catch-all graph `@_` has no entry, so a load into it is never skipped as unchanged and always merges.

The comparison is per graph name. The same file loaded under two names produces two graphs, and the load does not detect this. Elements without an identifier follow the rules of RFC-003 on a merge.

### 2.6 Visibility in queries

From the third phase, meta nodes appear in every query that scans nodes, in counts and in Cypher exports. Queries that want data nodes only exclude them by membership of `@@`. The loader does not hide them.

### 2.7 Export

The exporter reads the metadata separately from data elements. The record is built from the graph's name (as `name`) and the ordinary properties; the digest describes the source file, not the graph, and is not written.

- No entry: no top-level record is written.
- One graph exported: its record is written first in the file, followed by a blank line.
- More than one entry in the database, with no graph chosen for the export: the export fails, before any output, with an error listing them.
- A meta node, in the third phase, that has labels other than `Graph` or has relationships is an export error listing the offenders.

The record has one property per line, indented by two spaces, with keys sorted by Unicode code point, and values spelled as the value-mapping decision determines:

```
{
  name: "roster",
  revision: 4
}
```

The record always includes `name`, so an exported file names its graph whatever the original file name was.

## 3. Alternatives Considered

3.1. **Copy the record's properties onto every node.** Rejected: it multiplies the data by the node count, the copies can diverge after edits, and a graph with no nodes cannot hold a record.

3.2. **Store the record as one JSON or text property on a node.** Rejected: its fields could not be queried or edited as properties.

3.3. **Hide the meta node from queries.** Rejected: Relate's public interface cannot do it, and the constitution requires depending on that interface only. Placing meta nodes in their own graph makes the separation a matter of membership instead.

3.4. **A reserved label such as `_GramMeta` as the marker.** Rejected: once meta nodes belong to the meta graph, membership is the marker, and `Graph` says what the node is.

3.5. **Name the meta graph `meta`.** Rejected: a user could name a graph `meta`. Reserving `@@` keeps it from colliding with a user's name.

3.6. **One meta node per database, merged across loads.** Rejected: a second load would mix or overwrite the first record's properties. One entry per graph keeps each graph's information apart.

3.7. **Fail on any repeated graph name, without a digest.** Rejected: an unchanged file would fail on reload, where a digest comparison makes it a harmless no-op.

3.8. **Hash the normalized content of the parsed graph.** Rejected: it needs a canonical form that does not depend on element identifiers, which the export does not guarantee (ADR-002). Raw bytes are exact and cheap, and a reformatted file is treated as changed.

3.9. **Require every graph to be named.** Rejected: loading a small fixture from standard input with no name would then fail for no benefit.

3.10. **Store the metadata in meta nodes in every phase.** Rejected for the first two phases: meta nodes appear in every query that scans nodes, and the first phase needs no per-graph metadata. They return in the third phase as a materialization.

3.11. **Keep the metadata only in the ontology document, with no meta nodes.** Deferred: it cannot be listed or queried with Cypher, and the ontology document is a preview feature. The third phase adds the nodes.

## 4. Consequences

- Named graphs need `RELATE_PREVIEW=ontology` in the second phase, and are unavailable where it cannot be set, such as WebAssembly builds.
- A user who sets an ontology of their own replaces the single ontology document and erases every graph's entry. The write also cannot share a transaction with the data load, and creating the document where none exists produces a stub the user did not write (RFC-003 question 7.18).
- From the third phase the meta node is visible in `MATCH (n)` queries and counts. Users and tools exclude it by membership of `@@`; this has to be documented.
- Loading the same file under two different names creates two graphs with duplicated elements and no warning.
- A reformatted or re-commented file has a different digest and counts as changed.
- A graph name with a dot can only come from the record or the caller. A file `roster.v2.gram` is read as graph `roster`, kind `v2`, and fails (ADR-003).
- Exports always write `name`, so a file that omitted `name` gains it on a round trip.
- A file read from standard input with no record and no name loads into the catch-all graph `@_`, which has no digest of its own.
- `Graph` is an ordinary label in gram files and is not reserved. A file cannot set `@id`, so it cannot forge a meta node.
- Open: the record key that names a graph (`name` or `namespace`), the file-name fallback, and the reserved graph names (RFC-003 questions 7.12 to 7.14).
- Follow-on decisions: the shape of an entry in the ontology document, how elements record membership of a named graph (RFC-003), and the CLI flag for the graph name, specified with the CLI integration.

## 5. Related

- RFCs: RFC-001, RFC-003
- ADR-003 covers the first phase's handling of the record and the file kind.
