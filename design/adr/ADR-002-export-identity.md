---
id: ADR-002
title: Export Identity
status: accepted
rfcs: [RFC-001, RFC-003]
created: 2026-11-06
---

# ADR-002: Export Identity

> The amendments of 2026-11-08 (stored `@id` written as the identifier, shared settings with the load) took effect on 2026-11-08, when ADR-004 (which amends Constitution III) was accepted.
>
> Amended 2026-11-09: §2.5 now says how a label is written (with ADR-008), and §2.6 how a leading slash in a quoted name is written (ADR-006 §2.5).

## 1. Context

A gram file refers to an element by an identifier written on it: `(n0)-[r0:KNOWS]->(n1)` only means something because `n0` and `n1` were declared elsewhere in the file. An exporter has to supply those identifiers for every element it writes. The database cannot supply them:

- Relate's element identity is internal. It is not a stable name for an element, and the same graph loaded into two databases can end up with different internal values.
- A loaded gram file's own identifiers are kept as `@id`, so an export can write them back. Elements without one, such as anonymous elements and elements created through Cypher, still need an identifier from elsewhere.
- Many graphs already carry a natural identifier as an ordinary property, such as a `uuid` or `email`, and writing that value as the gram identifier makes the exported file easy to read and compare.

An exported file also has to load back correctly. A reload treats all identifiers in a file as one space shared by nodes and relationships. Two elements with equal identifiers are one element, and an element may be defined only once, so an exporter that wrote a duplicate identifier would produce a file that fails to load. And because exports are meant to be diffed, the same database has to produce the same file every time.

The identifier syntax constrains what can be written. In the tree-sitter grammar for gram, an identifier is a symbol (a letter or underscore followed by letters, digits, `_`, `.`, `-` or `@`), an integer (`-?(0|[1-9][0-9]*)`), or a backtick-quoted name whose text may contain any character except an unescaped backtick, backslash or newline. A plain symbol cannot start with `@`.

The parent RFC (RFC-001) sets the direction: export takes an identifier from a `@id` value (the identifier the loader stored, ADR-001), a unique property value, or a generated sequence, in that order, and fails when a configured property is missing or not unique. It leaves the naming scheme for generated identifiers to a decision. The scheme, the configuration, the ordering and the quoting are fixed below.

## 2. Decision

### 2.1 Modes and configuration

Export has three identity modes, selected by an `identity` setting. The default is `auto`.

| Mode | Identifier of an element |
|---|---|
| `generate` | Always generated (2.3). |
| `property` | The value of a configured property when its kind (node or relationship) has one (2.2), otherwise generated. `@id` values are ignored. A kind with no setting is generated as a whole; a kind with a setting is never mixed. |
| `auto` | The element's `@id` value if it has one, otherwise the configured property for its kind, otherwise generated. Unlike `property`, `auto` needs no setting. |

Two optional settings name the property to use: `node-id-property` and `relationship-id-property`. Each applies to one kind of element. In `property` mode at least one of them MUST be set, or the export fails with a configuration error. A property setting given in `generate` mode is also a configuration error.

These are the same two settings that a load uses to take identity from a property (ADR-001), so a graph exported with a property setting reloads by matching that property. Exporting and loading with different settings duplicates elements.

`property` exists so that an export can ignore `@id` values and produce the same output as before named graphs were in use.

Elements loaded with an identifier carry `@id` (ADR-001), so `auto` uses it for them and takes the configured property or a generated identifier for the rest.

### 2.2 Identifiers from a property

For each kind with a configured property, every element of that kind MUST have a value for the property, and the values MUST be usable and unique. The export otherwise fails, before any output, with an error that lists every offending element and why:

- **missing**: the element lacks the property;
- **unusable**: the value is not a string or an integer, or is an empty string;
- **duplicate**: the value's text equals another element's identifier.

The text of an identifier is the string itself, or the decimal form of an integer. Equality is by text, as in ADR-001, so the integer `42` and the string `"42"` are the same identifier. Uniqueness is checked across nodes and relationships together, since they share one identifier space on reload. The RFC speaks of uniqueness within a kind; the check here spans both. There is no per-element fallback to a generated identifier. The property stays in the exported record, so a reload still has it as an ordinary property. An `@id` value is held to the same checks, since Cypher edits can change it: it MUST be a string and is subject to the uniqueness test. An element with no `@id` is not an error; it takes its identifier from the property setting or a generated one (2.1). `@id` is written as the element's identifier and never as a record key.

### 2.3 Generated identifiers

Generated identifiers are `n0`, `n1`, … for nodes and `r0`, `r1`, … for relationships, counted from zero in export order (2.4). A candidate whose text is already in use by a property-derived or `@id` identifier is skipped, so a node property value `r0` does not collide with a generated relationship identifier.

### 2.4 Export order

Nodes are written in ascending order of Relate's internal id (`id(n)`), and so are relationships (`id(r)`). Relate's public library offers no enumeration interface other than Cypher through a connection, so the exporter reads nodes and relationships with queries that carry an explicit `ORDER BY` on the id, and the order does not depend on how the engine scans. Both reads run in one transaction, so they see one state of the database. Generated identifiers are assigned in that order. Records are written with labels and property keys sorted by Unicode code point, so the text of an element does not depend on how Relate stores its labels and properties.

The loader creates elements in order of first appearance (ADR-001). A load followed by an export is therefore stable if Relate assigns ids deterministically for a given sequence of creations; a test covers this.

### 2.5 Layout

Every element is given an identifier, a refinement of the RFC's "referenced more than once" (see 3.6). The file consists of the top-level record (the properties record that may open a gram file), if any, then every node on its own line, then every relationship on its own line, with a blank line between sections:

```
(n0:Person {name: "Ada"})
(n1:Person {name: "Grace"})

(n0)-[r0:KNOWS {since: 1843}]->(n1)
```

Nodes are written with their full record. A label is written as a bare symbol if it matches `[A-Za-z_][0-9A-Za-z_.@-]*`, and otherwise as a backtick-quoted name with the escapes of 2.6 (added with ADR-008, which applies the same rule to relationship types). Relationships refer to their endpoints by identifier alone, in their stored direction. Paths are not produced. The spelling of property values, including quoting of strings, is decided by the value mapping (ADR-006). The first phase writes no top-level record (ADR-003). Later phases write it from the graph's metadata (ADR-005).

### 2.6 Writing an identifier

An identifier's text is written as a bare integer if it matches `-?(0|[1-9][0-9]*)`, as a bare symbol if it matches `[A-Za-z_][0-9A-Za-z_.@-]*`, and otherwise as a backtick-quoted name. Inside a quoted name, a backtick is written `` \` ``, a backslash `\\`, a newline `\n`, a carriage return `\r` and a tab `\t`. Text that starts with `@` is quoted, since a plain symbol cannot start with it, and so is text equal to `true` or `false`. Other control characters are written raw. A `/` in the leading run of blanks and slashes of a quoted name is written `\/`, for the reason given in ADR-006 §2.5.

### 2.7 Failure behavior

An export is all-or-nothing: every error, whether about identity or about a value that has no gram form, is detected before the first byte of output is produced, and the error lists all offending elements, not only the first.

A property key starting with `@` other than `@id` on any exported element is also an export error, listed with the other offenders, since a reload rejects such keys (ADR-001).

## 3. Alternatives Considered

3.1. **Use Relate's internal identities as identifiers (`n<id>`).** Rejected: the numbers would expose engine internals and show gaps after deletions, so the file would record accidents of history. Dense counters do not.

3.2. **UUIDs or random identifiers.** Rejected: every export of the same database would differ, defeating diffing.

3.3. **Identifiers derived from a hash of an element's content.** Rejected: elements with identical content would collide, and the identifiers would be opaque and change whenever a property changes.

3.4. **Fall back to a generated identifier for each element whose property is missing or duplicated.** Rejected: the file would mix two kinds of identifier without saying which is which, and the choice of which element keeps the duplicated value is arbitrary. An error lets the user fix the data or switch mode.

3.5. **Remove the identifier property from the exported record.** Rejected: a reload would lose the property, and the exported file would no longer describe the same data. Keeping it also lets a reload recover the value.

3.6. **Write identifiers only on elements referenced more than once.** Rejected: identifiers would appear and disappear as edges are added or removed, making diffs noisy, and a file with some elements named and some not is harder to edit by hand.

3.7. **Emit relationships as chained paths.** Rejected: a graph has many valid chainings, so a stable order would need a rule for choosing among them, and it makes lines longer. One relationship per line is deterministic and diffs by line.

3.8. **Canonical ordering by sorting elements by content.** Rejected: elements can have identical content, so the order would still need a tie-breaker, and sorting every property of every element is costly for large graphs. Internal order is cheap and is reproducible for a given database.

3.9. **One identifier setting for both nodes and relationships.** Rejected: relationships rarely carry a property that suits as an identifier, so a single setting would make every such export fail.

3.10. **Stream output and report errors midway.** Rejected: an error after output has started leaves a truncated file that may still parse as valid gram, so a reader cannot tell it from a complete export.

3.11. **Bare integers (`0`, `1`, …) as generated identifiers.** Rejected: nodes and relationships share one identifier space, so they would need one continuous sequence, and the letter prefixes make the kind visible when reading a file.

## 4. Consequences

- Generated identifiers are not stable under edits: deleting or inserting an element shifts the numbers after it, and a diff of two exports of an edited database can show many changed lines. Property-based identifiers do not have this effect.
- Two databases holding the same graph, built in different orders, can export in different orders. Exports are reproducible for a given database, not canonical across databases.
- The relative order of ids is whatever Relate assigns. The documentation describes `id()` only as the internal id and does not promise that ids ascend in creation order or are never reused after deletion, so the order is reproducible for a given database but carries no other meaning.
- A file that was loaded and then exported keeps the identifiers it wrote. Anonymous elements get generated ones, which can differ from one export to the next as the graph changes.
- Every element gets an identifier, so exports are larger than the minimum.
- Labels are written sorted, so a reload can reorder a node's labels compared with the source file.
- Relate returns a query result as a whole, so a large export holds every node and relationship row in memory before writing.
- A database that holds `@`-prefixed property keys other than `@id` cannot be exported until they are given a meaning.
- Reporting every offending element before writing requires a pass over the graph before the first byte is written.
- Exporting in `property` mode and reloading with default settings duplicates elements, since the identities do not line up.
- Elements that carry the same identifier in two exports, as generated identifiers `n0` and `r0` always do, are one element when both files are loaded into the first phase's single identifier space (ADR-001).
- Named graphs (RFC-003) extend this decision and do not replace it.
- Follow-on decisions: the spelling of values on export (ADR-006), graph metadata and the top-level record (ADR-005), and the library settings and CLI flags for `identity`, `node-id-property` and `relationship-id-property`, specified with the CLI integration.

## 5. Related

- RFCs: RFC-001
