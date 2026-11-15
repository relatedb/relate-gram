---
description: "Task list for gram-value as the Value Model"
---

# Tasks: gram-value as the Value Model

**Input**: Design documents from `/specs/004-gram-value-model/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, quickstart.md (all present)

**Tests**: Required. Constitution VI (test-first, non-negotiable): each phase writes its tests first and confirms they fail, or pin current behavior, before the code they cover.

**Organization**: Grouped by user story in the order the work needs them: the loader's value type first (US1, US4), then the exporter (US2), then the slash condition (US3), then documents (US5). The behavior is decided by ADR-013 (ADR-012, ADR-011 and ADR-006 where unchanged); commits and the pull request cite the ADR section a task implements (Constitution I).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an incomplete task)
- **[Story]**: US1 to US5, from spec.md
- Paths are relative to the repository root

---

## Phase 1: Setup

- [X] T001 Re-measure the baseline: `cargo test --release --test perf -- --ignored --nocapture`; record the typed load time and ratio in a "Baseline" note at the end of this file; if it differs from 348.7 ms by more than 10%, use the new figure for SC-004 (research R6)
- [X] T002 [P] Run `cargo test` and confirm everything is green before any change, so later failures are this change's

**Checkpoint**: baseline recorded; tests green.

---

## Phase 2: User Story 1 - Same loads, same rejections (Priority: P1) 🎯 MVP

**Goal**: the loader holds `gram_value::Value` with positions; nothing visible changes.

**Independent Test**: all fixtures and the position tests pass unchanged.

### Tests for User Story 1 (pin current behavior first)

- [X] T003 [US1] Add tests in `tests/contract/load_values.rs`: `-0`, `[-0]` and a record value `-0` store 0; a decimal beyond float precision (`0.1000000000000000055511151231257827`) loads as the nearest float; `1e400`-style overflow and a non-zero literal that rounds to zero fail with `value.decimal`; an array of a `date` and a `datetime` fails with `value.array-mixed` at the second element and the message contains `datetime` and not `tagged` (ADR-013 section 2.2). They pass today; they pin the behavior through the refactor
- [X] T004 [P] [US1] Add a unit test module in `src/values.rs` (compiled with the refactor) that lowers each accepted kind of `tests/fixtures/valid/values-*.gram` (use the existing file names) and checks the resulting `gram_value::Value` and spans; write it against the target API, so it fails to compile until T006

### Implementation for User Story 1

- [X] T005 [US1] Change `src/typed/mod.rs`: `Tag::validate` returns `Result<String, String>` (the normal content); remove `TypedValue`, `TypedContent` and `PointContent`; update the unit tests in that file (`text()`, `a_point_carries_its_numbers` becomes a test that the point's normal content is `SRID=4979;1.5 -2 3`) (ADR-013 section 2.3)
- [X] T006 [US1] In `src/values.rs`, replace `PropValue` with `Lowered { value: gram_value::Value, span: Span, items: Vec<Span> }`; make `convert_value` and `convert_record` build `Value`s per the table of ADR-013 section 2.2 (`-0` rule, `integer_from_digits`, hexadecimal locally, `decimal_from_text` with the `BigDecimal` re-parse, `Value::tagged` for recognized tags, string for unrecognized tags with the existing warning or error); keep every diagnostic code, subject and span; the array homogeneity rule compares `Value::kind()` and the tag for tagged elements, and the message names the tag (FR-001 to FR-004)
- [X] T007 [US1] In `src/plan.rs`, change `PlannedNode.props`, `PlannedRel.props` and `value_text` to the new type (`value_text` accepts a string or integer value as before)
- [X] T008 [US1] In `src/load.rs`, replace `to_value` with the single conversion of ADR-013 section 2.4 (integer, decimal, boolean, string, array; any other kind returns an error); filter typed values with a helper that checks for a `Tagged` value or an array of them; keep the rest of the plan-writing code (FR-006)
- [X] T009 [US1] Run `cargo test`; every test passes except those that need T010 (the typed write step reads the old content type)

**Checkpoint**: the crate builds with the typed write step stubbed in T010; non-typed loading is unchanged.

---

## Phase 3: User Story 4 - Typed values and points still load and export (Priority: P1)

**Goal**: the typed write step reads from `Value::Tagged`.

**Independent Test**: typed fixtures and typed contract and round-trip tests pass unchanged.

- [X] T010 [US4] In `src/load.rs`, rewrite `typed_tag`, `typed_param`, `group_spans`, `typed_text` and `typed_failure` to read tag and content from `Value::Tagged`; `typed_param` builds the point parameter map from `gram_value::tags::interpret_tag("point", content)` and passes the content string for text tags; `typed_text` returns the content for every tag (research R3, FR-005)
- [X] T011 [US4] Run the typed contract, round-trip and position tests (`load_typed`, `export_typed`, `check`, `positions`) and the typed perf case; all pass with no expectation changed
- [X] T012 [US4] Add a test in `tests/contract/load_typed.rs` that a rejected typed list element is reported at that element (an unknown zone in the second element of a datetime array) and that a rejected point-bearing group still reports a `value.tag-content` at a point's span (ADR-013 section 2.3)

**Checkpoint**: typed behavior unchanged; `TypedValue`, `TypedContent` and `PointContent` no longer exist.

---

## Phase 4: User Story 2 - Strings written as canonical text (Priority: P1)

**Goal**: the exporter writes every property value with `Value::to_text()`.

**Independent Test**: string shapes round-trip; non-string output is byte-identical.

### Tests for User Story 2 (write first, confirm they fail)

- [X] T013 [P] [US2] Update the golden export strings that show symbol-like strings quoted: `tests/contract/export_basic.rs` (lines 15-16, 93), `export_values.rs` (32-34, 48, `strs: [a, b]`), `export_identity.rs` (59, 100, 109, `email: x@y`) and `tests/cli.rs` (143-144, 193-194, the `normalize` expectations); leave every other expectation untouched (ADR-013 section 2.6, SC-003)
- [X] T014 [P] [US2] Create `tests/roundtrip/strings.rs` (add `mod strings;` to `tests/roundtrip/main.rs`): export then reload a fixed list of strings (symbols `active`, `a-b`, `foo.bar`, `e5`, `a-1.5`, `x1`; `Ada Lovelace`; `true`, `false`; empty; `42`, `4x`; `café`; strings with `"`, `\`, newline, tab, backspace, form feed) and assert the exported text matches gram-value's canonical text for each (bare or quoted as stated) and the reloaded value equals the original (spec User Story 2, SC-002)
- [X] T015 [P] [US2] Add tests in `tests/contract/export_values.rs`: a string with NUL, as a property and inside a list, fails the export and is listed (kept); an empty list is `[]`; a nested list and a mixed list fail with element and key (ADR-013 section 2.5); integers, floats (`-0.0`, `1e21` written positionally), booleans and lists of each export exactly as before
- [X] T016 [US2] Run the new and updated tests; confirm the string tests fail because the exporter still quotes every string

### Implementation for User Story 2

- [X] T017 [US2] In `src/export.rs`, add the mapping of ADR-013 section 2.5 from a Relate property value to `Result<gram_value::Value, String>` (research R4) and write each property as `names::name(key): <value.to_text()>`; keep the NUL, nested-list, mixed-list, non-finite and unsupported-SRID errors with the element and key; map any `unwritable` diagnostic to an `export.value` error as a fallback (FR-007, FR-008)
- [X] T018 [US2] Route UUID, point and temporal values through `Value::tagged` with the same content as before (UUID lower-case text, point text of ADR-012 section 2.6, Relate's text for temporals) and delete `scalar_text`, `float_text`, `temporal_value_text` and the `Temporals` special case if nothing else uses them (FR-010, FR-011)
- [X] T019 [US2] Run `cargo test`; T013 to T015 pass and no expectation other than those of T013 changed (SC-001, SC-003)

**Checkpoint**: exports follow canonical text; the one visible change is bare symbol strings.

---

## Phase 5: User Story 3 - Strings that begin with a slash (Priority: P1)

**Goal**: drop the leading-slash escape for string values, provided the round trip holds.

**Independent Test**: the deterministic round-trip over slash shapes.

- [X] T020 [US3] Add to `tests/roundtrip/strings.rs`: strings that begin with `//`, with `/** x */ y` (a later `*/`), with leading blanks then `/`, and with `/` inside; each as a property value; and keys, labels, relationship types and identifiers beginning with `/`; export, reload, assert equal values; for values, assert no `\/` appears in the text; for names, assert the escape is still present (ADR-013 section 2.6)
- [X] T021 [US3] Run it: the value assertions on the escape fail while the string path is canonical text only if the exporter still escapes; if T017 already dropped the escape (gram-value does not write it), the test passes and shows the round trip holds with tree-sitter-gram 0.7.0
- [X] T022 [US3] If any shape fails to reload equal, stop: restore the escape for string values (a local wrapper around `to_text` for the leading run), record the failing shape in the spec, and report to the user; otherwise continue
- [X] T023 [US3] Remove `names::string_literal` and the string-value tests of `src/names.rs` that cover it; keep `escape_into` and the name tests (FR-009, FR-011)

**Checkpoint**: slash strings round-trip; names keep their escape.

---

## Phase 6: User Story 5 - Documents and the library break (Priority: P2)

- [X] T024 [P] [US5] `CHANGELOG.md` Unreleased: under Changed, list the library items removed or changed against 0.1.0 (`values::PropValue`, `plan::PlannedNode.props`, `PlannedRel.props`, `plan::value_text`, `names::string_literal`) and that symbol strings are now exported bare; adjust the Added entry that mentions `PropValue::Typed` and the `typed` module (ADR-013 section 4)
- [X] T025 [P] [US5] `specs/001-phase1-load-export/data-model.md` (lines 22, 24) and `contracts/library-api.md` (line 89): replace `PropValue` with the new type and add the note "amended by ADR-013"
- [X] T026 [P] [US5] `specs/002-typed-tags/{research,data-model,plan}.md` and `specs/003-gram-value-tags/spec.md` (and its plan and data model if they name the removed types): note the replacement types with "amended by ADR-013"; do not rewrite history in tasks files
- [X] T027 [US5] Search for stale mentions: `grep -rn "PropValue\|string_literal\|TypedContent\|TypedValue\|PointContent" src tests docs README.md specs` and fix or justify each hit; check `docs/supported-gram.md` and `README.md` for any exported-output example with a quoted symbol string and update it
- [X] T028 [US5] Confirm ADR-006's header shows the ADR-013 amendment (already added) and that `design/adr/README.md` lists ADR-013 as accepted

**Checkpoint**: documents match behavior.

---

## Phase 7: Polish and Verification

- [X] T029 `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `scripts/check-public-api.sh`, full `cargo test`, `cargo tree` unchanged (SC-005, SC-006)
- [X] T030 Re-run the performance check; SC-004 requires at most 1.2 times the T001 figure; record the result at the end of this file; if exceeded, reconsider the second interpretation of points (ADR-013 section 4) before continuing
- [X] T031 Walk `quickstart.md` by hand and record the results in the pull request
- [ ] T032 Tick every completed task, update `checklists/requirements.md`, and open the pull request citing ADR-013 (do not merge or push without the user's answer)

---

## Dependencies and Order

- Phase 1, then Phase 2 (T005 before T006; T006 to T008 in order; they compile together), Phase 3 after Phase 2, Phase 4 after Phase 3, Phase 5 after Phase 4 (the slash result depends on the exporter), Phase 6 and 7 last.
- Parallel: T002; T004; T013 to T015; T024 to T026.

## Implementation Strategy

MVP is Phases 1 to 3: the loader on gram-value with unchanged behavior. Phase 4 delivers the visible change; Phase 5 is the condition that lets it drop the escape. Each phase leaves the suite green.

## Baseline

Before the change (release build, 2026-11-15): 10,000 nodes, plain load 253.9 ms, typed load 377.3 ms (ratio 1.49). That is within 10% of the 348.7 ms recorded after ADR-012, so the SC-004 ceiling stays 1.2 x 348.7 = about 418 ms.

After the change: plain load 259.5 ms, typed load 344.5 ms (ratio 1.33), typed export 201.8 ms. Below the ceiling, so SC-004 holds; the second interpretation of points costs nothing measurable.

## Results

- Slash condition (T020 to T022): the deterministic round trip passes with tree-sitter-gram 0.7.0 for every shape (`//x`, `/** x */ y`, leading blanks, interior slashes), so the string escape is gone; names keep it and round-trip too.
- The only golden export strings that changed are those showing a symbol-like string quoted (`tests/cli.rs`, `export_basic.rs`, `export_values.rs`, `export_identity.rs`); every other expectation is unchanged (SC-003).
- `cargo test` (13 test targets), `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `scripts/check-public-api.sh` pass; `cargo tree` shows no new dependency (SC-005, SC-006).
- Quickstart (T031): all scenarios behave as written.
