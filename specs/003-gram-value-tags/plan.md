# Implementation Plan: Known Tags from gram-value

**Branch**: `adopt-gram-value` | **Date**: 2026-11-15 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/003-gram-value-tags/spec.md`

## Summary

Implement ADR-012. `Tag::validate` stops parsing text itself: it calls `gram_value::tags::interpret_tag`, maps a diagnostic to `value.tag-content`, and applies a small profile (offset on `time` and `datetime`; supported SRIDs, dimensions and latitude on `point`). The text tags pass gram-value's normal content string to Relate; a point is built from the numbers of gram-value's `Point`, as before. The exporter writes points as `SRID=n;x y [z]` with shortest decimals. `src/typed/{temporal,point,uuid}.rs` are deleted after a recorded comparison of old and new validators over the fixtures. `PropValue`, the lowering in `values.rs`, the load plan and the diagnostics contract keep their shape.

## Technical Context

**Language/Version**: Rust, edition 2021, toolchain 1.96.0 (`rust-toolchain.toml`); `rust-version` 1.90 (gram-value needs 1.75)

**Primary Dependencies**: `relate` ~1.0, `tree-sitter` 0.25, `tree-sitter-gram` =0.7.0, `thiserror`, and new `gram-value` 0.1 (no runtime dependencies of its own). Research R1.

**Storage**: none of its own; unchanged

**Testing**: `cargo test`: unit tests for the profile, contract tests for load and export, round-trip, position and corpus tests, and a one-time comparison test of old against new validators (research R4). Constitution VI: each task group starts with failing tests.

**Target Platform**: native (macOS and Linux); unchanged

**Project Type**: library plus the `relate-gram` command; no new option

**Performance Goals**: SC-004: loading 10,000 typed elements takes at most 1.2 times the time measured before the change, from the baseline recorded in task T001 with `tests/perf.rs`

**Constraints**: public Relate interfaces only (Constitution V); `scripts/check-public-api.sh` unchanged (gram-value is not Relate); no `unsafe`; a load completes or changes nothing

**Scale/Scope**: about 700 lines removed, about 150 added (profile, adapter, point writer); five source files touched (`typed/mod.rs`, `values.rs`, `load.rs`, `export.rs`, `Cargo.toml`)

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle or constraint | Status | Basis |
|---|---|---|
| I. Design before code | Pass | ADR-012 passed `/adr-review` and is accepted; this plan and spec name it; commits cite it |
| II. Strict subset, loud failures | Pass | Every bad typed value stays a located `value.tag-content`; unrecognized tags keep drop-and-report; the one rejected-to-accepted change (no seconds) is spelling only and listed in the spec |
| III. Identity is preserved | Pass | Typed values stay unusable as identity (unchanged); `@id` untouched |
| IV. Round-trip fidelity | Pass | Points reload equal; SC-002 and the round-trip tests cover each type |
| V. Public interfaces only | Pass | No new Relate item; `gram-value` is a separate crate |
| VI. Test-first | Pass, enforced in `tasks` | Failing tests and fixtures precede each change |
| Rust constraints | Pass | Constitution 2.2.0 permits `gram-value` (ADR-012); no `unsafe`; toolchain pinned |
| Workflow | Pass | Feature branch; ADR reviewed; PR before merge |

No violations; Complexity Tracking is empty. Re-checked after Phase 1: unchanged.

## Project Structure

### Documentation (this feature)

```text
specs/003-gram-value-tags/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── checklists/requirements.md
└── tasks.md             # produced by /speckit-tasks
```

No `contracts/`: the diagnostics contract of `specs/002-typed-tags/contracts/diagnostics.md` keeps its codes; only message examples change, and task T022 updates them.

### Source Code (repository root)

```text
Cargo.toml           # gram-value = "0.1"
src/
├── typed/
│   ├── mod.rs       # Tag, TypedContent, TypedValue stay; validate() = interpret + profile
│   ├── profile.rs   # NEW: offset rule, SRID/dimension/latitude rule, message wording
│   ├── temporal.rs  # DELETED after the comparison (T012)
│   ├── point.rs     # DELETED
│   └── uuid.rs      # DELETED
├── values.rs        # reports the new messages; otherwise unchanged
├── load.rs          # typed_param/typed_text read the new content; point numbers from gram-value
└── export.rs        # point_text writes SRID=n;x y [z] with shortest decimals
tests/
├── typed_compare.rs # NEW, temporary: old validators (copied under tests/) against new, over fixtures
├── fixtures/        # point and time fixtures updated; new fixtures for week, weekDate, no-seconds
├── contract/export_typed.rs, load_typed.rs, tests/perf.rs   # updated
docs/supported-gram.md, CHANGELOG.md, specs/002-typed-tags/*, design/adr/ADR-011 (done)
```

**Structure Decision**: single crate as before; the validators collapse into one adapter plus a profile module, so the loader and exporter call sites keep their current shape.

## Complexity Tracking

None.
