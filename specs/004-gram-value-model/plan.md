# Implementation Plan: gram-value as the Value Model

**Branch**: `adopt-gram-value-model` | **Date**: 2026-11-15 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/004-gram-value-model/spec.md`

## Summary

Implement ADR-013. The loader's `PropValue` becomes `gram_value::Value` wrapped with positions (`Lowered`: the value, its span, and an array's element spans). The tree-sitter lowering in `src/values.rs` keeps every rejection and builds `Value`s; a typed value is `Value::tagged(tag, normal_content)` and the typed write step reads tag and content from it, interpreting a point again for its numbers. The exporter converts each Relate property value to a `Value` and writes it with `to_text()`; elements, identifiers, labels, types and keys stay with `names`. A string value loses the escape of a leading slash once a deterministic round-trip test proves tree-sitter-gram 0.7.0 reads such strings. `TypedValue`, `TypedContent`, `PointContent`, `names::string_literal`, `scalar_text`, `float_text` and `temporal_value_text` are removed.

## Technical Context

**Language/Version**: Rust, edition 2021, toolchain 1.96.0; `rust-version` 1.90

**Primary Dependencies**: unchanged: `relate` ~1.0, `tree-sitter` 0.25, `tree-sitter-gram` =0.7.0, `thiserror`, `gram-value` 0.1

**Storage**: none of its own; unchanged

**Testing**: `cargo test`: the contract, round-trip, position and corpus tests are the safety net (SC-001); added unit tests for the lowering, the exporter mapping and a deterministic slash round-trip. Constitution VI: failing tests first.

**Target Platform**: native; unchanged

**Project Type**: library plus the `relate-gram` command; no new option

**Performance Goals**: SC-004: typed load of 10,000 elements at most 1.2 times 348.7 ms (release, same machine), re-measured in task T001 before changes

**Constraints**: public Relate interfaces only (Constitution V); loads complete or change nothing; no `unsafe`; export of non-string values byte-identical (FR-010)

**Scale/Scope**: touches `src/values.rs`, `plan.rs`, `load.rs`, `export.rs`, `names.rs`, `typed/mod.rs`; golden strings in four test files; documents named in ADR-013 section 4

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle or constraint | Status | Basis |
|---|---|---|
| I. Design before code | Pass | ADR-013 passed an independent review and is accepted; spec and plan name it; commits cite it |
| II. Strict subset, loud failures | Pass | The lowering keeps every rejection, code and position (FR-002); the one new acceptance (`-0` already accepted) is unchanged behavior; no value is approximated |
| III. Identity is preserved | Pass | `@id`, identity properties and `value_text` behave as before; typed values stay unusable as identity |
| IV. Round-trip fidelity | Pass | User Stories 2 and 3 test export then load for every string shape and slash case |
| V. Public interfaces only | Pass | No new Relate item; `gram-value` is not Relate |
| VI. Test-first | Pass, enforced in `tasks` | Each phase writes failing tests before the code |
| Rust constraints | Pass | Constitution 2.2.0 covers `gram-value`; no new dependency |
| Workflow | Pass | Feature branch; ADR reviewed; PR before merge |

No violations; Complexity Tracking is empty. Re-checked after Phase 1: unchanged.

## Project Structure

### Documentation (this feature)

```text
specs/004-gram-value-model/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── checklists/requirements.md
└── tasks.md             # produced by /speckit-tasks
```

No `contracts/`: `specs/001-phase1-load-export/contracts/library-api.md` changes and is edited in task T031.

### Source Code (repository root)

```text
src/
├── values.rs        # Lowered { value, span, items }; convert_value/convert_record build Values; to_relate conversion
├── typed/mod.rs     # Tag::validate returns the normal content string; TypedValue/TypedContent/PointContent removed
├── plan.rs          # PlannedNode/PlannedRel.props: Vec<(String, Lowered)>; value_text over Value
├── load.rs          # typed write step reads tag and content from Value::Tagged; point re-interpreted
├── export.rs        # property values: Relate value -> gram_value::Value -> to_text; old value writers removed
└── names.rs         # string_literal removed; escape_into stays for backticked names
tests/
├── contract/export_basic.rs, export_values.rs, export_identity.rs, tests/cli.rs   # golden strings
├── contract/load_values.rs   # -0, big decimal, array-mixed message
├── roundtrip/strings.rs      # NEW: deterministic slash and string-shape round trip
docs: CHANGELOG.md, specs/001-*/{data-model,contracts/library-api}.md, specs/002-typed-tags/*, specs/003-gram-value-tags/spec.md
```

**Structure Decision**: single crate; the lowered value and the exporter mapping stay in the files that own them today.

## Complexity Tracking

None.
