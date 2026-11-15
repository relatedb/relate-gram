---
description: "Task list for Known Tags from gram-value"
---

# Tasks: Known Tags from gram-value

**Input**: Design documents from `/specs/003-gram-value-tags/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, quickstart.md (all present)

**Tests**: Required. Constitution VI (test-first, non-negotiable): each phase writes its tests and fixtures first and confirms they fail before the code they cover.

**Organization**: Grouped by user story, in the order the work needs them: the comparison (US6) comes right after the adapter, because it needs the old validators, which it then lets us delete. The behavior is decided by ADR-012 (ADR-011 and ADR-006 where unchanged); commits and the pull request cite the ADR section a task implements (Constitution I).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an incomplete task)
- **[Story]**: US1 to US6, from spec.md
- Paths are relative to the repository root
- An invalid fixture has a sibling `.expect` file whose line is `code line column offset` (see `tests/fixtures/README.md`)

---

## Phase 1: Setup

- [ ] T001 Record the performance baseline before any change: run `cargo test --release --test perf -- --ignored --nocapture` and `cargo bench --bench load_export`, and paste the typed-load and plain-load timings and their ratio into a "Baseline" note at the end of this file (research R6, SC-004)
- [ ] T002 Add `gram-value = "0.1"` to `[dependencies]` in `Cargo.toml`; check `cargo tree` shows exactly one new package and `cargo build` passes (ADR-012 section 2.1, FR-010)
- [ ] T003 [P] Run `scripts/check-public-api.sh` and confirm it still passes with the dependency added

**Checkpoint**: crate builds with the dependency; baseline recorded.

---

## Phase 2: Foundational (blocks every user story)

**Purpose**: the profile and the adapter that replace the validators' calls. Tests first.

- [ ] T004 Read `/Users/abk/Developer/gram-data/gram-value/rust/src` for the exact `Diagnostic` field names and the `Point`, `Time`, `DateTime` shapes; note them in `research.md` R2 (closes its open check)
- [ ] T005 Write failing unit tests in `src/typed/profile.rs` (new file, add `mod profile;` to `src/typed/mod.rs`) for each rule of `data-model.md`: `time` and `datetime` without an offset fail with a message about the offset; with `Z`, `+hh:mm` and a zone name plus offset they pass; a point with no SRID fails (message says Relate needs an SRID); SRID 4326 and 7203 with two coordinates and 4979 and 9157 with three pass; an unsupported SRID or a dimension disagreeing with the SRID fails; latitude outside [-90, 90] fails for 4326 and 4979 and is not checked for 7203 and 9157; `date`, `duration`, `uuid`, `localtime`, `localdatetime` pass (ADR-012 section 2.3)
- [ ] T006 Implement `src/typed/profile.rs` so the tests of T005 pass: `pub(super) fn check(interpretation: &gram_value::tags::Interpretation) -> Result<(), String>` (FR-002)
- [ ] T007 Write failing tests in `src/typed/mod.rs` for `Tag::validate`: it returns gram-value's message without the `"{tag}: "` prefix for `date`, `duration` and `uuid` faults; `TypedContent::Text` holds the normal content (`17:35Z` gives `17:35:00Z`, `+00:00` gives `Z`, a fraction `.50` gives `.5`, a UUID in upper case gives lower case, a zone name kept); a point gives `TypedContent::Point` with srid, x, y, z from gram-value (ADR-012 section 2.2, FR-001, FR-004)
- [ ] T008 Rewire `Tag::validate` in `src/typed/mod.rs`: call `gram_value::tags::interpret_tag(self.name(), text)`, map a diagnostic to the stripped message, apply `profile::check`, build `TypedContent` from the interpretation; leave `temporal`, `point` and `uuid` in place, unused by `validate`; tests of T007 pass

**Checkpoint**: unit tests pass; the old modules still compile (and keep their own unit tests); some contract tests now fail where ADR-012 changes the outcome (the next phases fix them).

---

## Phase 3: User Story 6 - The old and new validators are shown to agree (Priority: P2)

**Goal**: evidence that replacing the validators changed only what ADR-012 names; then the old code goes.

**Independent Test**: the comparison output equals the Expected Differences table.

- [ ] T009 [US6] Copy the current `src/typed/temporal.rs`, `point.rs` and `uuid.rs` unchanged into `tests/typed_compare/old/` as a snapshot (with a small `mod.rs` exposing `date`, `time`, `localtime`, `datetime`, `localdatetime`, `duration`, `point`, `uuid`) and add `tests/typed_compare/main.rs`, a test target of its own
- [ ] T010 [US6] Write the comparison in `tests/typed_compare/main.rs`: gather every input of `tests/fixtures/valid/typed-*.gram` and `tests/fixtures/invalid/tag-content-*.gram` (parse the tagged strings out with the crate's own `syntax` module) plus the accepted and rejected forms listed in `specs/002-typed-tags/data-model.md` and `research.md` (as an explicit array in the test); run the snapshot and `Tag::validate`; print one line per input where accept/reject differs (`cargo test --test typed_compare -- --nocapture`)
- [ ] T011 [US6] Run it; replace the "Expected Differences" table of `specs/003-gram-value-tags/spec.md` with the observed differences (input, before, after, reason); any difference not explained by ADR-012 stops the work and goes back to the user
- [ ] T012 [US6] Delete `src/typed/temporal.rs`, `point.rs` and `uuid.rs`, their `mod` lines and the now-unused helpers; delete `tests/typed_compare/` after copying its input list into `tests/fixtures/` as fixtures where an input had none; `cargo build` passes (FR-008, SC-005; check `git diff --stat` shows at least 600 lines removed from `src/typed/`)

**Checkpoint**: the differences are recorded in the spec; the old validators are gone.

---

## Phase 4: User Story 1 - Points in the general form (Priority: P1) 🎯 MVP

**Goal**: points load and export as `SRID=n;x y [z]`.

**Independent Test**: round-trip a file with a point of each supported SRID.

### Tests for User Story 1 (write first, confirm they fail)

- [ ] T013 [P] [US1] Update `tests/fixtures/valid/typed-point.gram`, `typed-all.gram` and `typed-lists.gram` to the new point form, with a two-dimensional and a three-dimensional point of each SRID (4326, 7203, 4979, 9157) (ADR-012 section 2.4)
- [ ] T014 [P] [US1] Update the `tag-content-point-*` fixtures under `tests/fixtures/invalid/` and their `.expect` files: add `tag-content-point-wkt` (old extended well-known text) and keep `no-srid`, `bad-srid`, `dimension`, `latitude`, `exponent`; positions are the tagged string's
- [ ] T015 [US1] Update the point assertions in `tests/contract/export_typed.rs` and `tests/contract/load_typed.rs`: expected text `SRID=4326;12.5 55.7`, `-74` not `-74.0`, `-0` exported as `0`, and a reload equal to the original (ADR-012 section 2.6)
- [ ] T016 [P] [US1] Update the point in `tests/perf.rs` (`typed_gram`, about line 81) to the new form
- [ ] T017 [US1] Confirm the tests of T013 to T016 fail for the right reason (export still writes extended well-known text)

### Implementation for User Story 1

- [ ] T018 [US1] In `src/load.rs`, make `typed_param` read the point numbers from `TypedContent::Point` as filled in T008 (no change if T008 already produced them) and make `typed_text` return the normal content text; the point stays built with Cypher `point({...})` (ADR-012 section 2.2)
- [ ] T019 [US1] In `src/export.rs`, change `point_text` to write `point`SRID={srid};{x} {y}[ {z}]`` with each coordinate formatted by a new `coordinate_text` (Rust `Display` for `f64`, `-0` written `0`, non-finite still an error); `float_text` stays for plain floats (research R5, FR-006)
- [ ] T020 [US1] Run the contract, round-trip and position tests; the tests of T013 to T016 pass

**Checkpoint**: points round-trip in the new form; invalid point forms are rejected.

---

## Phase 5: User Story 2 - A time without seconds loads (Priority: P1)

**Goal**: the seconds rule is gone, the offset rule stays.

**Independent Test**: load `time`17:35Z`` and compare with `time`17:35:00Z``.

### Tests for User Story 2 (write first, confirm they fail)

- [ ] T021 [P] [US2] Move `tests/fixtures/invalid/tag-content-time-no-seconds.*` to `tests/fixtures/valid/typed-time-no-seconds.gram` (also a `datetime`, `localtime` and `localdatetime` without seconds); keep `tag-content-time-no-offset` and `tag-content-datetime-no-offset` as invalid
- [ ] T022 [US2] Add a contract test in `tests/contract/load_typed.rs`: ``time`17:35Z` `` and ``time`17:35:00Z` `` store equal values; `+00:00` and `Z` store equal values; a fraction `.50` and `.5` store equal values; an upper-case UUID stores the same value as lower case (ADR-012 section 2.2)

### Implementation for User Story 2

- [ ] T023 [US2] Run the tests; if any fails, fix the adapter or profile of T006 and T008 (no new rule is expected)

**Checkpoint**: seconds are optional; offsets are required.

---

## Phase 6: User Story 3 - Bad values, unchanged code and position (Priority: P1)

**Goal**: every remaining invalid fixture still fails with the same code and position, and messages are clean.

**Independent Test**: all `tests/fixtures/invalid/tag-content-*` expectations hold.

- [ ] T024 [US3] Run the full invalid-fixture suite; every fixture not named in the Expected Differences table keeps its `.expect`; correct any fixture whose position moved only if the table says so
- [ ] T025 [P] [US3] Add a contract test in `tests/contract/load_errors.rs` that the message of a bad date contains the tag name once and no `"date: date"` repetition, and that a load with a bad value leaves a database with known content unchanged (ADR-011 section 2.3, spec scenario 2)
- [ ] T026 [P] [US3] Keep the zone-name behavior: `check` accepts an unknown zone name, load rejects it; verify with the existing `tag-content-datetime-zone-*` fixtures and one added unknown-zone test in `tests/contract/load_typed.rs`

**Checkpoint**: error behavior is as before apart from the listed differences.

---

## Phase 7: User Story 4 - `week` and `weekDate` stay unrecognized (Priority: P2)

- [ ] T027 [P] [US4] Add fixtures `tests/fixtures/valid/tag-week-dropped.gram` and `tag-weekdate-dropped.gram` (strings stored, a drop warning) and `tests/fixtures/invalid/tag-week-error-policy.gram` if the fixture harness supports a policy, otherwise a contract test in `tests/contract/load_values.rs` for `--tags error` (spec User Story 4)
- [ ] T028 [US4] Run the tests; `Tag::from_name` already excludes both names, so no code change is expected (ADR-012 section 2.5)

**Checkpoint**: unrecognized-tag behavior is demonstrated for the two new names.

---

## Phase 8: User Story 5 - Documentation and everything else (Priority: P2)

- [ ] T029 [P] [US5] Update `docs/supported-gram.md`: the point row and example, the `time` example of `value.tag-content` (an offset, not seconds, is the rule), a note that `week` and `weekDate` are not recognized (FR-011)
- [ ] T030 [P] [US5] Update `CHANGELOG.md` [Unreleased]: typed tags as listed, points as `SRID=n;x y [z]`, files using extended well-known text need rewriting, the corpus's `time`17:35Z`` now loads and `point`30 10`` still does not, dependency on `gram-value`
- [ ] T031 [P] [US5] Update `specs/002-typed-tags/` data-model, quickstart, research, `contracts/diagnostics.md` and tasks wherever they show the old point form or the seconds rule, each with a short note that ADR-012 amended it
- [ ] T032 [US5] Add a test in `tests/fix.rs` that `fix` leaves a point in extended well-known text unchanged and annotates the error (ADR-012 section 2.4, ADR-010 section 4)
- [ ] T033 [US5] Search the repository for stale mentions: `grep -rn "POINT(" docs specs README.md CHANGELOG.md tests src` and `grep -rn "seconds" docs specs/002-typed-tags`; fix or justify each hit

**Checkpoint**: documentation matches behavior.

---

## Phase 9: Polish and Verification

- [ ] T034 `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `scripts/check-public-api.sh`, full `cargo test`
- [ ] T035 Re-run the performance check of T001 and record the ratio against the baseline; SC-004 requires at most 1.2 times; if exceeded, profile and report before continuing
- [ ] T036 Walk `quickstart.md` by hand (steps 1 to 5) and record the results in the pull request
- [ ] T037 Tick every completed task here, update `specs/003-gram-value-tags/checklists/requirements.md`, and open the pull request citing ADR-012 (do not merge or push without the user's answer)

---

## Dependencies and Order

- Phase 1 then Phase 2; Phase 3 needs Phase 2; Phases 4 to 7 need Phase 3 and can follow in order (4 and 5 touch the same fixtures directory and tests, so they run in sequence); Phase 8 needs 4 and 5; Phase 9 last.
- Parallel: T003; T013, T014, T016; T021; T025, T026; T029 to T031.

## Implementation Strategy

MVP is Phases 1 to 4: points in the general form with validators replaced. Phases 5 to 8 are small and complete the change; the comparison in Phase 3 is the gate before any deletion.

## Baseline

(To be filled by T001.)
