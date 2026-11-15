# Implementation Plan: The embedding Tag

**Branch**: `embedding-tag` | **Date**: 2026-11-15 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/005-embedding-tag/spec.md`

## Summary

Implement ADR-014. `embedding` becomes a ninth `typed::Tag`; its content is validated by `gram_value::tags::interpret_tag` plus a Relate profile that rejects a dimension above 4096 and then the first non-finite element. The loader keeps a recognized embedding as `Value::tagged("embedding", normal_content)` (ADR-013) and builds a Relate vector in a follow-up statement per element kind, key and dtype, with the kind as a literal keyword; a probe before any write finds out whether Relate accepts 16-bit kinds. The exporter turns a float `Value::Vector` into `Embedding::to_value` and refuses an integer one. A new module converts 16-bit elements and is tested over every bit pattern. Documents named in ADR-014 section 2.6 are amended.

## Technical Context

**Language/Version**: Rust, edition 2021, toolchain 1.96.0; `rust-version` 1.90

**Primary Dependencies**: unchanged: `relate` ~1.0, `tree-sitter` 0.25, `tree-sitter-gram` =0.7.0, `thiserror`, `gram-value` 0.2 (PR 10). No new dependency (FR-009, SC-007)

**Storage**: none of its own; unchanged

**Testing**: `cargo test`. New: fixtures under `tests/fixtures/{valid,invalid}` with `.expect` files, contract tests for load, check and export, a round-trip test per dtype, an exhaustive 16-bit conversion test. Constitution VI: failing tests first. Two integration-test targets because the preview flag is process-wide: one with `RELATE_PREVIEW=float16-coordinate-types` set before the first connection, one without.

**Target Platform**: native; unchanged

**Project Type**: library plus the `relate-gram` command; no new option

**Performance Goals**: SC-006: 1,000 vectors of dimension 1,536 loaded and exported, first measured when the feature is complete and held to 1.2 times that figure afterwards; SC-005: the typed 10,000-element load stays within its existing ceiling

**Constraints**: public Relate interfaces only (Constitution V); loads complete or change nothing; no `unsafe`; export of everything but a float vector unchanged (FR-012)

**Scale/Scope**: touches `src/typed/mod.rs`, `src/typed/profile.rs`, a new `src/typed/elements.rs`, `src/values.rs`, `src/load.rs`, `src/export.rs`, `src/diagnostics.rs` (no new code), `docs/supported-gram.md`, `CHANGELOG.md`, `CLAUDE.md`, and the amended decisions of ADR-014 section 2.6

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle or constraint | Status | Basis |
|---|---|---|
| I. Design before code | Pass | ADR-014 passed an independent review and is accepted; spec and plan name it; commits cite it |
| II. Strict subset, loud failures | Pass | Every unstorable embedding is a located error (FR-004 to FR-006); no element is dropped or rounded; a 16-bit kind is never loaded as another kind (ADR-014 section 3.7) |
| III. Identity is preserved | Pass | Identity properties behave as before; an embedding cannot be one |
| IV. Round-trip fidelity | Pass | User Story 2 and the exhaustive 16-bit test show bytes survive export then load |
| V. Public interfaces only | Pass, with a note | The vector's kind type is not exported by `relate`, so the exporter reads the kind from the `Debug` name of the public field; a test creates every kind and fails if a name changes (R5) |
| VI. Test-first | Pass, enforced in `tasks` | Each phase writes failing tests before the code |
| Rust constraints | Pass | Constitution 2.2.0 covers `gram-value`; no new dependency |
| Workflow | Pass | Feature branch; ADR reviewed; PR before merge |

No violations; Complexity Tracking is empty. Re-checked after Phase 1: unchanged.

## Project Structure

### Documentation (this feature)

```text
specs/005-embedding-tag/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── checklists/requirements.md
└── tasks.md             # produced by /speckit-tasks
```

No `contracts/`: `specs/001-phase1-load-export/contracts/library-api.md` gains the `Tag::Embedding` variant and is edited in the documents task.

### Source Code (repository root)

```text
src/
├── typed/mod.rs        # Tag::Embedding; ALL has nine; validate unchanged in shape
├── typed/profile.rs    # embedding arm: dimension above 4096, then first non-finite element
├── typed/elements.rs   # NEW: widen/narrow payload bytes per dtype; f16 and bf16 conversions
├── values.rs           # array of embeddings is one value.tag-content error at the array
├── load.rs             # TypedGroup gains the dtype; typed_param widens; typed_expr builds vector(...); probe before write_plan
├── export.rs           # Value::Vector -> Embedding::to_value; integer kinds are export.value errors
tests/
├── fixtures/valid/embedding-*.gram, fixtures/invalid/embedding-*.gram (+ .expect)
├── contract/load_embedding.rs, export_embedding.rs
├── roundtrip/embedding.rs
├── embedding16.rs      # NEW target, flag on: exhaustive conversion, 16-bit load and export
├── embedding16_off.rs  # NEW target, flag off: probe failure before any write
docs: docs/supported-gram.md, CHANGELOG.md, CLAUDE.md, ADR-010/011 headers, RFC-002 question 7.3
```

**Structure Decision**: single crate; the element conversion gets its own file under `typed/` because it is database-free and used by the profile, the loader and the exporter.

## Complexity Tracking

None.
