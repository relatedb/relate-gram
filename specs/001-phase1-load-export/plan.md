# Implementation Plan: Phase 1 Gram Load and Export

**Branch**: `001-phase1-load-export` | **Date**: 2026-11-09 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/001-phase1-load-export/spec.md`

## Summary

A Rust library crate, `relate-gram`, that loads gram text into a Relate database and exports a Relate database as gram text, as the spec describes. Load parses the text with `tree-sitter` and `tree-sitter-gram` (ADR-007), turns the syntax tree into an in-memory load plan that applies the identity, value and relationship rules of ADR-001, ADR-003, ADR-006 and ADR-008, and then writes the plan through Relate's public library interface in one transaction after matching against the database with a single scan. Export reads the database in one transaction in id order and writes gram text under ADR-002 and ADR-006. All errors are values that carry a stable code and a source range, so that tests can assert positions (Constitution VI) and a fixer can be built later (design note of 2026-11-09). Research decisions are in [research.md](research.md).

## Technical Context

**Language/Version**: Rust, edition 2021, toolchain pinned to 1.96.0 in `rust-toolchain.toml`; `rust-version` 1.90 (research R2)

**Primary Dependencies**: `relate` (git dependency on `relatedb/relate`, pinned to a commit); `tree-sitter` 0.25; `tree-sitter-gram` 0.6.1; `thiserror`. Dev: `proptest` (round-trip properties), `toml` and `serde` (the corpus expectations)

**Storage**: none of its own; writes to and reads from a caller-supplied Relate database (in-memory in this phase)

**Testing**: `cargo test`: contract tests for load and export, round-trip tests, error-position tests, and a corpus runner over the `tree-sitter-gram` submodule (Constitution VI)

**Target Platform**: native (macOS and Linux); WebAssembly and language bindings are not targeted (ADR-007 consequences)

**Project Type**: library (one crate; a command-line integration comes later and is not part of this feature)

**Performance Goals**: SC-006: 10,000 elements load and export in under 10 seconds; measured by a benchmark in this feature (research R5)

**Constraints**: public Relate interfaces only (Constitution V); a load or export either completes or changes nothing (FR-001); no `unsafe`

**Scale/Scope**: tested to 50,000 elements, the scale SPIKE-001 measured; beyond that is untested

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle or constraint | Status | Basis |
|---|---|---|
| I. Design before code | Pass | The spec and this plan name ADR-001, 002, 003, 004, 006, 007 and 008 and do not restate them; commits cite the ADR they implement |
| II. Strict subset, loud failures | Pass | Every rejection is a located diagnostic (data model, `contracts/diagnostics.md`); comments and `kind` are the documented exceptions (ADR-007 §2.5, ADR-003) |
| III. Identity is preserved | Pass | `@id` handling and identity-property settings follow ADR-001 §2.7 and §2.10 and ADR-002 |
| IV. Round-trip fidelity | Pass | Round-trip tests and the `tree-sitter-gram` corpus runner are in the test plan |
| V. Public interfaces only | Pass | Only `Database`, `Connection`, `Value` and `QueryResult` from `relate` are used (research R1, R4) |
| VI. Test-first | Pass, enforced in `tasks` | Each task group starts with its failing tests |
| Rust constraints | Pass | Dependencies are those ADR-007 names plus `relate`; toolchain pinned; no `unsafe`; `fmt` and `clippy` run in CI |
| Workflow | Pass | Work is on a feature branch; ADR-006, 007 and 008 passed `/adr-review` before acceptance |

No violations; Complexity Tracking is empty. Re-checked after Phase 1: unchanged.

## Project Structure

### Documentation (this feature)

```text
specs/001-phase1-load-export/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── library-api.md
│   └── diagnostics.md
└── tasks.md             # produced by /speckit-tasks
```

### Source Code (repository root)

```text
Cargo.toml
rust-toolchain.toml
external/tree-sitter-gram/    # git submodule at v0.6.1; corpus source only
src/
├── lib.rs                    # public API: load, export, settings, report, diagnostics
├── diagnostics.rs            # Span, Diagnostic, codes
├── settings.rs               # LoadSettings, ExportSettings, TagPolicy, IdentityMode
├── syntax.rs                 # parse bytes; collect ERROR/MISSING; typed node views
├── values.rs                 # ADR-006 mapping, load side and export side
├── names.rs                  # identifier, label, type and key quoting (ADR-002 §2.6, ADR-006 §2.5, ADR-008 §2.4)
├── plan.rs                   # LoadPlan built from the tree; identifier resolution (ADR-001, ADR-008)
├── load.rs                   # scan, match, batch writes, transaction
└── export.rs                 # read in id order, identifiers, layout
tests/
├── contract/                 # main.rs plus one file per user story (a folder target needs main.rs)
├── positions/                # main.rs; one case per fixture-testable diagnostic code, asserting line, column, offset
├── roundtrip/                # main.rs; fixtures and proptest properties
├── corpus.rs                 # tree-sitter-gram corpus runner, with support/ reader and corpus_expectations.toml
├── perf.rs                   # ignored test asserting SC-006
├── syntax.rs                 # syntax-layer tests
└── fixtures/                 # valid/ and invalid/ gram files
benches/load_export.rs        # SC-006
examples/load_and_query.rs    # quickstart
```

**Structure Decision**: one library crate with flat modules. The parsing layer (`syntax`), the rules (`plan`, `values`, `names`) and the database layer (`load`, `export`) are separate modules so that the rules can be tested without a database. A command-line binary and the bindings are not created here.

## Complexity Tracking

No violations to justify.
