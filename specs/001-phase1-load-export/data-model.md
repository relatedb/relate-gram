# Data Model: Phase 1 Gram Load and Export

The values the library passes between its layers. Names are conceptual; the contract for the public ones is in [contracts/library-api.md](contracts/library-api.md). Rules are in the ADRs named; this file lists the shapes only.

## Source

- **Span**: a range of the source: start and end byte offsets, plus the 1-based line and column of the start (column counts bytes from the line start). Every diagnostic and every plan element carries one.

## Diagnostics

- **Diagnostic**: `code` (stable string, [contracts/diagnostics.md](contracts/diagnostics.md)), `severity` (error or warning), `span`, `message`, `related` (zero or more labelled spans, such as the first definition of a duplicate).
- A load returns diagnostics in source order. Plan-phase errors are all returned together; a matching error found against the database is returned with them.

## Settings

- **LoadSettings**: `node_id_property` and `relationship_id_property` (each absent by default; ADR-001 §2.10), `tags` (`warn` or `error`, default `warn`; ADR-006 §2.3), `max_diagnostics` (cap on listed errors and positions, default 100; ADR-007 §2.2).
- **ExportSettings**: `identity` (`auto`, `generate` or `property`, default `auto`), `node_id_property`, `relationship_id_property` (ADR-002 §2.1).

## Load

- **LoadPlan**: the file's elements in first-appearance order, with all file-scope rules already applied.
  - **PlannedNode**: `key` (identifier text, or none if anonymous), `labels` (in written order), `properties` (ordered map of key to **PropValue**), `span` of its definition, `defined` (definition or reference only).
  - **PlannedRelationship**: `key` (or none), `type`, `start` and `end` (indexes of planned nodes), `properties`, `span`, `defined`.
- **PropValue**: `Int(i64)`, `Float(f64)`, `Bool(bool)`, `Str(String)` or `List` of one of these kinds (ADR-006 §2.1, §2.2). Nothing else; every other gram value is a diagnostic before a PropValue exists.
- **RecordInfo**: the top-level record's keys and spans, used for the not-stored warning (ADR-003 §2.2). No values are kept.
- **IdentityTable**: built in the loader from one scan of the database: identifier text to the internal id and kind (node or relationship) of each existing element, with the type and endpoint ids of relationships. Duplicates are recorded for the error of ADR-001 §2.7.
- **LoadReport**: the diagnostics (warnings on success, errors on failure), and counts: nodes created, nodes updated, relationships created, relationships updated, elements created without an identifier (ADR-004), referenced-only nodes created (ADR-001 §2.3), tags dropped (ADR-006 §2.3).

## Export

- **ExportGraph**: nodes and relationships read in id order, each with its internal id, labels or type, properties as `relate::Value`, and for relationships the endpoint ids.
- **ExportElement**: an element with its chosen identifier text (ADR-002 §2.1 to §2.3), labels or type sorted (ADR-002 §2.4), properties sorted by key, ready to be written.

## Relationships between layers

`bytes` → (syntax) tree or diagnostics → (plan) `LoadPlan` or diagnostics → (load) database writes and `LoadReport`.
`Connection` → (export) `ExportGraph` or diagnostics → `ExportElement`s → text.

## State transitions

None persist in the library. A load moves the caller's connection from not-in-transaction to not-in-transaction: it begins, then commits or rolls back, and leaves no open transaction on return.
