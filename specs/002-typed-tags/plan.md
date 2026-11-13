# Implementation Plan: Typed Tags for Gram Values

**Branch**: `typed-tags-spec` | **Date**: 2026-11-13 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/002-typed-tags/spec.md`

## Summary

Implement ADR-011: eight recognized tags (`date`, `time`, `localtime`, `datetime`, `localdatetime`, `duration`, `point`, `uuid`) turn a tagged string into a typed property value on load, typed lists load from arrays of one recognized tag, and typed values export as tagged strings. The loader gains a validator per content form (so `check` works without a database), a new diagnostic code `value.tag-content`, and a follow-up write step that builds typed values through Cypher constructors. The exporter reads typed values from the property map (UUIDs and points directly, temporals through `toString` in the same query) and writes the ADR's forms. Vectors and the other runtime values stay unwritable. Empty lists are already done and are not touched.

## Technical Context

**Language/Version**: Rust, edition 2021, toolchain 1.96.0 (`rust-toolchain.toml`); `rust-version` 1.90

**Primary Dependencies**: `relate` 1.0.x (`~1.0`), `tree-sitter` 0.25, `tree-sitter-gram` =0.7.0, `thiserror`. No new dependencies: the validators are small hand-written parsers, and no date or UUID crate is needed (research R5).

**Storage**: none of its own; writes to and reads from a caller-supplied Relate database

**Testing**: `cargo test`: contract tests for load and export, round-trip tests, position tests, the corpus runner, and a typed-value fixture set (one valid and one invalid file per form). Constitution VI: each task group starts with its failing tests.

**Target Platform**: native (macOS and Linux); unchanged

**Project Type**: library plus the `relate-gram` command (the command needs no new option)

**Performance Goals**: SC-004: a 10,000-element file with typed values on every element loads in no more than three times the plain-string time; measured by an added case in `tests/perf.rs` and `benches/load_export.rs`

**Constraints**: public Relate interfaces only (Constitution V); a load either completes or changes nothing; no `unsafe`; typed values are built with Cypher constructors (ADR-011 section 2.7, research R2)

**Scale/Scope**: tested as before to 50,000 elements; typed-value density beyond SC-004 is untested

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle or constraint | Status | Basis |
|---|---|---|
| I. Design before code | Pass | The spec and this plan implement ADR-011 (accepted after `/adr-review`) and name it; commits cite it |
| II. Strict subset, loud failures | Pass | Every bad typed value is a located `value.tag-content`; unrecognized tags keep their documented drop-and-report; nothing is approximated (data model, `contracts/diagnostics.md`) |
| III. Identity is preserved | Pass | A typed value is not usable as identity, like a list today, and is reported as `id.property-missing` (research R4); `@id` handling is unchanged |
| IV. Round-trip fidelity | Pass | SC-001 and the round-trip tests cover every type; export stays deterministic |
| V. Public interfaces only | Pass | Only items `relate` re-exports are used: `Value`, `Temporal` (as a matched type), `uuid_hyphenated`, and the public fields of a matched point; `scripts/check-public-api.sh` stays in the checks (research R1, R2) |
| VI. Test-first | Pass, enforced in `tasks` | Fixtures and failing tests precede each implementation task |
| Rust constraints | Pass | No new dependency; toolchain pinned; no `unsafe`; `fmt` and `clippy` in CI |
| Workflow | Pass | Work is on a feature branch; ADR-011 passed `/adr-review` before acceptance; the plan goes through a PR |

No violations; Complexity Tracking is empty. Re-checked after Phase 1: unchanged.

## Project Structure

### Documentation (this feature)

```text
specs/002-typed-tags/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   └── diagnostics.md
└── tasks.md             # produced by /speckit-tasks
```

### Source Code (repository root)

```text
src/
├── values.rs        # PropValue gains a Typed variant; recognized tags are matched in convert_value
├── typed/           # NEW: the typed-value model and the eight content-form validators (no database)
│   ├── mod.rs       #   Tag, TypedValue, dispatch by tag
│   ├── temporal.rs  #   date, time, localtime, datetime, localdatetime, duration
│   ├── point.rs     #   extended well-known text
│   └── uuid.rs
├── plan.rs          # typed values are excluded from identity; typed items collected for the write step
├── load.rs          # follow-up statements per (kind, key, tag) after the basic properties
├── export.rs        # typed values written as tagged strings; one added column for temporal text
├── diagnostics.rs   # value.tag-content added to code::ALL
└── fix.rs           # no new fix; value.tag-content is annotated (ADR-010)
tests/
├── fixtures/valid/typed-*.gram       # NEW, one per type plus lists
├── fixtures/invalid/tag-content-*.gram, array-mixed-tag-*.gram   # NEW, with .expect files
├── contract/load_typed.rs, export_typed.rs                       # NEW
├── roundtrip/                        # fixtures list extended
└── perf.rs, benches/load_export.rs   # typed-density case
docs/supported-gram.md, README.md, CHANGELOG.md                   # updated (FR-012)
```

**Structure Decision**: one new module directory, `typed`, holds everything about the eight forms that does not need a database (parsing, range checks, normalization), so it is testable alone and shared by `values` (load) and `export`. The database-facing code stays in `load` and `export`.

## Complexity Tracking

_(none)_
