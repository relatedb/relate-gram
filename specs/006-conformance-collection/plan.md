# Implementation Plan: The Conformance Collection and its Rust Runner

**Branch**: `accept-adr-015` | **Date**: 2026-11-16 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/006-conformance-collection/spec.md`

## Summary

Implement ADR-015 on the Rust side. A `conformance/` directory holds `reject`, `load`, `export` and `fix` cases converted from the existing fixtures, `.expect` files, golden strings and fixer examples. A test runner shared by two test targets (`tests/conformance.rs` with the 16-bit preview flag unset, `tests/conformance16.rs` with it set) reads `case.gram` and `expected.gram` with `relateby-gram`, runs each case against the library through its public interface, compares by the canonical-line multisets of ADR-015 section 2.3, probes capabilities, applies known failures and writes a results file. The collection checks (vocabulary, versions, byte-exact files, unique identifiers, coverage of diagnostic codes, `gram lint --strict`) are a third test target plus a CI step. The tests the collection replaces are removed in the same change.

## Technical Context

**Language/Version**: Rust, edition 2021, toolchain 1.96.0; `rust-version` 1.90

**Primary Dependencies**: unchanged for the library. New dev-dependency: `relateby-gram` 0.7 (ADR-015 section 2.10; constitution 2.3.0)

**Storage**: none of its own; the runner opens an in-memory `relate` database per case

**Testing**: three new test targets under `tests/` sharing a module: `conformance.rs`, `conformance16.rs`, `conformance_checks.rs`. Constitution VI: failing tests first, in the order of `tasks.md`

**Target Platform**: native; unchanged. CI gains a step that installs the `gram` tool and runs the lint

**Project Type**: library plus the `relate-gram` command; no behavior change

**Performance Goals**: SC-006: the runner stays within twice the time of the tests it replaces, measured in task T001 before the conversion

**Constraints**: public Relate interface only (Constitution V; the runner uses `relate::{Connection, Database, Value}` and the crate's public API); no `unsafe`; no behavior change to the library or the command (FR-012)

**Scale/Scope**: 71 reject cases, 25 + 4 load cases, the export and fix cases converted from the contract tests and `tests/fix.rs`, several hundred small files; touches `tests/`, `conformance/`, `.github/workflows/ci.yml`, `.gitattributes`, `Cargo.toml` (dev-dependency), `src/lib.rs` (test-support re-export), `CLAUDE.md` and `docs/`

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle or constraint | Status | Basis |
|---|---|---|
| I. Design before code | Pass | ADR-015 is accepted; spec and plan name its sections; commits cite it |
| II. Strict subset, loud failures | Pass | The collection pins every rejection with code and position (User Story 1); no library behavior changes |
| III. Identity is preserved | Pass | Expected graphs use identifiers as ADR-015 section 2.3 says; identity cases stay covered |
| IV. Round-trip fidelity | Pass | `load` cases with `exported.txt` reload the export and compare graphs |
| V. Public interfaces only | Pass | The runner reads values through Cypher and the public `Connection`; one doc-hidden re-export in `__test_support` exposes the 16-bit element conversion to tests (not a Relate item) |
| VI. Test-first | Pass, enforced in `tasks` | The runner's pieces are written against failing tests; each converted case is first shown to fail when its expectation is made wrong |
| Rust constraints | Pass | Constitution 2.3.0 allows `relateby-gram` in tests (ADR-015 section 2.10) |
| Workflow | Pass | Feature branch; ADR accepted; PR before merge |

No violations; Complexity Tracking is empty. Re-checked after Phase 1: unchanged.

## Project Structure

### Documentation (this feature)

```text
specs/006-conformance-collection/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── checklists/requirements.md
└── tasks.md             # produced by /speckit-tasks
```

No `contracts/`: the contract is `conformance/README.md`, written in task T052.

### Source Code (repository root)

```text
conformance/
├── README.md, VERSION, GRAMMAR_VERSION, vocabulary.gram, capabilities.gram, start.cypher
├── known-failures.gram          # empty at first
├── reject/<id>/  load/<id>/  export/<id>/  fix/<id>/     # case.gram, input.txt|bin, expected.gram, exported.txt, fixed.txt, start.cypher
.gitattributes                    # conformance/**/*.txt and *.bin are not text
tests/
├── conformance.rs                # pass 1: preview flag unset
├── conformance16.rs              # pass 2: preview flag set before the first connection
├── conformance_checks.rs         # collection checks
└── conformance/                  # shared module: collection, case reading, settings, value table, graph lines, probes, results
src/lib.rs                        # __test_support re-exports typed::elements::{widen, narrow}
.github/workflows/ci.yml          # installs the gram tool; runs the lint step
```

**Structure Decision**: one shared module used by three thin test targets, because the preview flag is process-wide and a process cannot run both passes.

## Complexity Tracking

None.
