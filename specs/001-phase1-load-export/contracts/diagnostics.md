# Contract: Diagnostics

Every error and warning has a stable `code` (listed here), a severity, a source span where the text is at fault, and a message. Tests assert the code and the span (Constitution VI). A code never changes meaning; a retired code is not reused. The column "ADR" names the rule that decides it.

## Input and syntax

| Code | Severity | Condition | ADR |
|---|---|---|---|
| `input.encoding` | error | input is not valid UTF-8 (span: the first bad byte) | ADR-007 §2.2 |
| `parse.error` | error | an ERROR node in the syntax tree | ADR-007 §2.2 |
| `parse.missing` | error | a MISSING node; the message names the expected kind | ADR-007 §2.2 |
| `construct.annotation` | error | an annotation | ADR-007 §2.4 |
| `construct.subject-pattern` | error | a bracketed subject pattern, or a bare reference in one | ADR-007 §2.4 |
| `construct.double-colon` | error | `::` before a label or property key | ADR-007 §2.4 |
| `construct.unsupported` | error | any node kind the loader does not read | ADR-007 §2.4 |

## Relationships

| Code | Severity | Condition | ADR |
|---|---|---|---|
| `rel.undirected` | error | an undirected arrow | ADR-008 §2.2 |
| `rel.bidirectional` | error | a bidirectional arrow | ADR-008 §2.2 |
| `rel.arrow-family` | error | a fat or wavy arrow | ADR-008 §2.2 |
| `rel.no-type` | error | a definition or bare arrow with no label | ADR-008 §2.3 |
| `rel.many-labels` | error | a definition with two or more labels | ADR-008 §2.3 |

## Values and keys

| Code | Severity | Condition | ADR |
|---|---|---|---|
| `value.range` | error | a range | ADR-006 §2.1 |
| `value.measurement` | error | a measurement | ADR-006 §2.1 |
| `value.map` | error | a nested map | ADR-006 §2.1 |
| `value.octal` | error | an octal literal | ADR-006 §2.1 |
| `value.integer-range` | error | an integer outside 64 bits | ADR-006 §2.1 |
| `value.decimal` | error | a decimal that is not finite or rounds to zero | ADR-006 §2.1 |
| `value.array-mixed` | error | an array of mixed kinds; the span is the first element that differs | ADR-006 §2.2 |
| `value.tag-dropped` | warning | a tag dropped under `TagPolicy::Warn`; one per tagged string | ADR-006 §2.3 |
| `value.tag` | error | a tagged string under `TagPolicy::Error` | ADR-006 §2.3 |
| `key.duplicate` | error | a repeated key; `related` holds the first | ADR-006 §2.4 |
| `key.empty` | error | an empty key | ADR-006 §2.4 |
| `key.reserved` | error | a key that starts with `@` | ADR-001 §2.8 |

## Identity and file

| Code | Severity | Condition | ADR |
|---|---|---|---|
| `id.kind-conflict` | error | an identifier used as both a node and a relationship; `related` holds the first use | ADR-001 §2.2 |
| `id.duplicate-definition` | error | a second definition; `related` holds the first | ADR-001 §2.3, §2.4 |
| `id.endpoints-differ` | error | a repeated relationship names different endpoints | ADR-001 §2.4 |
| `id.rel-never-defined` | error | a relationship identifier that is only referenced | ADR-001 §2.4 |
| `id.reference-only` | warning | a node identifier that is only referenced | ADR-001 §2.3 |
| `id.property-missing` | warning | a definition lacks the configured identity property | ADR-001 §2.10 |
| `match.kind-differs` | error | a stored element with this `@id` is the other kind | ADR-001 §2.7 |
| `match.type-differs` | error | a stored relationship has a different type or endpoints | ADR-001 §2.7 |
| `match.db-duplicate` | error | more than one stored element has the identifier; the message lists them | ADR-001 §2.7 |
| `file.kind` | error | the file kind or the record's `kind` is not `property-graph` | ADR-003 §2.1 |
| `file.record-not-stored` | warning | the record's properties are not stored; one warning listing them | ADR-003 §2.2 |
| `load.anonymous` | warning | the count of elements created without an identifier | ADR-004 |
| `load.in-transaction` | error | the connection is already in a transaction | contract |
| `load.database-error` | error | an error returned by Relate | contract |

## Export

| Code | Severity | Condition | ADR |
|---|---|---|---|
| `export.config` | error | a setting is invalid (for example `property` mode with no property) | ADR-002 §2.1 |
| `export.id-missing` | error | an element lacks the configured identity property | ADR-002 §2.2 |
| `export.id-unusable` | error | the identity value is not a string or integer, or is empty | ADR-002 §2.2 |
| `export.id-duplicate` | error | two elements share an identifier text | ADR-002 §2.2 |
| `export.value` | error | a value with no gram form; the message names which case | ADR-006 §2.6 |
| `export.key-reserved` | error | a property key that starts with `@` other than `@id` | ADR-002 §2.7 |
| `export.in-transaction` | error | the connection is already in a transaction | contract |

## Rules for spans

A span covers the construct named: the arrow for `rel.*`, the value for `value.*`, the key for `key.*`, the identifier for `id.*`. For a repeated or conflicting construct the span is the later one and `related` holds the earlier. Export diagnostics have no span; their message names the element by identifier and internal id.
