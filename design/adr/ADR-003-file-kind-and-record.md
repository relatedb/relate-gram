---
id: ADR-003
title: File Kind and the Top-Level Record
status: accepted
rfcs: [RFC-001, RFC-003]
created: 2026-11-06
---

# ADR-003: File Kind and the Top-Level Record

> Amended 2026-11-08. The meta node, graph naming and file digest that this ADR first decided moved to ADR-005, which is proposed. What remains is what the first phase of RFC-003 needs. The amendment took effect on 2026-11-08, when ADR-004 (which amends Constitution II and III) was accepted.

## 1. Context

A gram file may open with a record, a set of key-value properties written in braces before any node or relationship:

```
{
  name: "roster",
  revision: 4
}

(a:Person {name: "Ada"})
```

The record describes the file as a whole. A property graph has no element for that: nodes and relationships hold data about things in the graph, and nothing holds data about the graph itself. In the first phase of RFC-003 the loader loads into a single graph space, with no named graphs and no stored metadata, so it has nowhere to keep the record. The loader has to do something with it that does not lose information silently.

Several forces shape the choice:

- A gram file's top-level record also says what kind of file it is. In the wider gram ecosystem the `kind` property selects the semantics, conventions and style of the contents, for example `schema` for a file of constraints. Every gram file is valid data, but only one kind is a property graph.
- The ecosystem names files `graph-name.kind.gram`, so a file name can carry the kind as well as a name.
- Property keys starting with `@` are rejected on load (ADR-001), so a file cannot set an `@id` or any other key the loader reserves.
- The grammar allows a record's values to be maps and arrays, but only values that become Cypher property values can be stored (ADR-006).
- From the second phase on the record is stored as graph metadata and written back on export (ADR-005). A file that loads in the first phase should still load then, and an exported file carries a record from the second phase on.

The parent RFC (RFC-001) says the record is kept as a "meta node" and written back as the top-level record on export. RFC-003 stages that work, so the first phase needs only the rules in this ADR. Where the record is stored from the second phase on is decided in ADR-005.

## 2. Decision

### 2.1 One supported kind

The loader and exporter support one kind of gram file, `property-graph`. This is an assumption of the project, not a property of gram: other kinds exist in the ecosystem and are not loaded.

- A file whose name follows `graph-name.kind.gram` and whose kind is not `property-graph` is a load error.
- A top-level record with a `kind` property whose value is not the string `property-graph` is a load error that reports the position.
- The error message states the assumption: "relate-gram loads only kind 'property-graph'; this file is kind '<kind>'".
- A file with no kind, in its name or its record, is taken to be `property-graph`.

The `kind` property is checked and then dropped: it is not stored, and exports do not write it. The supported kind is a single constant.

### 2.2 The record in the first phase

- The record is parsed. Its property values are checked under the value mapping (ADR-006) like those of any element, and a value that cannot be stored is a load error that reports its position. A key starting with `@` is a load error (ADR-001).
- The loader reports a warning that lists the record's properties, `name` among them, as not stored. `kind` is checked (2.1) and is not listed.
- The record does not change the graph, and its `name` names nothing in this phase.
- An export writes no top-level record.

From the second phase on the record is kept as graph metadata (ADR-005).

## 3. Alternatives Considered

3.1. **Discard the record without a warning.** Rejected: information in the source file is lost without notice, which contradicts the strict-subset rule that nothing is dropped silently.

3.2. **Fail on a file that has a top-level record.** Rejected: an exported file carries one from the second phase on, so failing would reject every such file. A warning that lists what is not stored loses nothing silently.

3.3. **Copy the record's properties onto every node.** Rejected: it multiplies the data by the node count, the copies can diverge after edits, and a graph with no nodes cannot hold a record.

3.4. **Skip the value check on the record until it is stored.** Rejected: a file that loads in the first phase would then fail once the record is stored.

3.5. **Treat every dot-separated segment of a file name as a possible kind.** Rejected: names such as `v1.2` would be misread. Only the last segment is the kind.

3.6. **Accept `relate` as an alias for `property-graph`.** Rejected: a second name for one kind would have to be kept indefinitely. The kind is a single constant that can change in one place.

3.7. **Store `kind` with the record.** Rejected: only one kind is supported, so the value carries no information, and an exported file is the same kind whatever the original said.

## 4. Consequences

- A file with a top-level record loses it in the first phase, with a warning that lists its properties.
- The `kind` assumption is recorded here. The file-kind conventions of the wider gram ecosystem (for example `.schema.gram`) are not loaded, and the error says so.
- A file name with two or more dots is read as `graph-name.kind.gram`: `roster.v2.gram` has kind `v2` and fails.
- Exports never write `kind`, so a file that carried `kind: "property-graph"` loses it on a round trip. In the first phase an export writes no record at all.
- A record value that cannot be stored fails the load even though the record itself is not stored.
- Follow-on decisions: where the record is stored from the second phase (ADR-005), and the amendment of Constitution II for the reported drop of the record's properties (ADR-004).

## 5. Related

- RFCs: RFC-001, RFC-003
