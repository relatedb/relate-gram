---
description: "Task list for Typed Tags for Gram Values"
---

# Tasks: Typed Tags for Gram Values

**Input**: Design documents from `/specs/002-typed-tags/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/diagnostics.md, quickstart.md (all present)

**Tests**: Required. Constitution VI (test-first, non-negotiable): each phase writes its tests and fixtures first and confirms they fail before the implementation they cover.

**Organization**: Grouped by user story. The behavior is decided by ADR-011 (and ADR-006 where unchanged); commits and pull requests cite the ADR section a task implements (Constitution I).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an incomplete task)
- **[Story]**: US1 to US6, from spec.md
- Paths are relative to the repository root
- A fixture is a `.gram` file under `tests/fixtures/valid/` or `tests/fixtures/invalid/`; an invalid fixture has a sibling `.expect` file whose line is `code line column offset` (see `tests/fixtures/README.md`)

---

## Phase 1: Setup

**Purpose**: the code constant, the module skeleton and the branch of work, so every later task compiles.

- [ ] T001 Add `pub const VALUE_TAG_CONTENT: &str = "value.tag-content";` to `src/diagnostics.rs` and to `code::ALL`; extend the `code::ALL` uniqueness test if it lists codes (ADR-011 section 2.3)
- [ ] T002 [P] Create `src/typed/mod.rs` declaring `Tag` (`Date`, `Time`, `LocalTime`, `DateTime`, `LocalDateTime`, `Duration`, `Point`, `Uuid`) with `Tag::from_name(&str) -> Option<Tag>` matching the eight lower-case names exactly, and `Tag::name`; declare empty submodules `temporal`, `point`, `uuid`; add `pub(crate) mod typed;` to `src/lib.rs`
- [ ] T003 [P] Add the invalid-fixture codes used by this feature (`value.tag-content`, `value.array-mixed` on tags) to the fixture convention in `tests/fixtures/README.md`

**Checkpoint**: `cargo build` passes with the new module and code.

---

## Phase 2: Foundational (blocks every user story)

**Purpose**: the validators and the typed-value model, which load, check and export all share. Tests first.

- [ ] T004 [P] Write failing unit tests in `src/typed/temporal.rs` for `date`, `time`, `localtime`, `datetime`, `localdatetime`, `duration`: every accepted form of data-model.md ("`YYYY-MM-DD`, year of 4 or more digits, optional `-`"; "seconds present; offset `Z` or `+hh:mm`; datetime may add `[zone]`"; "upper-case `P`, at least one component, sign per number, fraction on seconds only") and every rejected form ("week, ordinal, basic and `+year` forms; month or day out of range, including 29 February in a common year"; "no offset; no seconds; second 60; a bare date as a datetime"; "an offset" on local forms; "`P` or `PT` alone; weeks; a leading `-` before `P`; fraction on other components; lower case"; a fraction of more than 9 digits), each with the fault message it must give
- [ ] T005 [P] Write failing unit tests in `src/typed/point.rs`: accepted `SRID=4326;POINT(-74.0 40.7)`, `SRID=7203;POINT(1 2)`, `SRID=4979;POINT Z(1 2 3)`, `SRID=9157;POINT Z(1 2 3)`; rejected: other SRIDs, a missing SRID, dimension and SRID disagreeing (a 2D SRID with `POINT Z` and the reverse), exponent form, non-finite values, "latitude outside [-90, 90]"; the parse returns srid and coordinates
- [ ] T006 [P] Write failing unit tests in `src/typed/uuid.rs`: accepted 8-4-4-4-12 hexadecimal digits in either case; rejected: braces, no hyphens, wrong grouping, non-hex digits
- [ ] T007 [P] Implement the validators in `src/typed/temporal.rs`: each returns `Result<TypedContent, String>` where the text is the normalized form passed to the constructor and the error is a fault message; "hours are 0 to 23, minutes and seconds 0 to 59, months 1 to 12, a day exists in its month (leap years included)"; a datetime's zone name is copied through unchecked; tests from T004 pass
- [ ] T008 [P] Implement the point parser in `src/typed/point.rs` returning srid, x, y and optional z; "a geographic latitude is within [-90, 90], and coordinates are finite"; tests from T005 pass
- [ ] T009 [P] Implement the UUID validator in `src/typed/uuid.rs`; tests from T006 pass
- [ ] T010 In `src/typed/mod.rs` add `TypedValue { tag, content, span }` (content is the normalized text, or the parsed point numbers) and `Tag::validate(&str) -> Result<TypedContent, String>` dispatching to the three submodules
- [ ] T011 Add `PropValue::Typed(TypedValue)` to `src/values.rs`; update `kind_name` (kind is the tag name), `convert_record` and every exhaustive match in `src/plan.rs`, `src/load.rs` and `src/fix.rs`; `plan::value_text` returns `None` for it, so a typed value is not usable as an identity and is reported as `id.property-missing` (research R4); make it compile with `unimplemented!`-free placeholders that existing tests still pass

**Checkpoint**: `cargo test` passes including the new unit tests; nothing yet loads a typed value.

---

## Phase 3: User Story 1 - A tagged string becomes a typed value on load (Priority: P1) 🎯 MVP

**Goal**: each recognized tag loads as the matching Relate type.

**Independent Test**: load one property of each tag into an empty database and check the stored types and values.

### Tests for User Story 1 (write first, confirm they fail)

- [ ] T012 [P] [US1] Create valid fixtures `tests/fixtures/valid/typed-date.gram`, `typed-time.gram`, `typed-localtime.gram`, `typed-datetime.gram` (including a zone name and nanoseconds), `typed-localdatetime.gram`, `typed-duration.gram` (including a negative and a mixed-sign duration), `typed-point.gram` (2D geographic and 3D Cartesian), `typed-uuid.gram` (upper-case input), and `typed-all.gram` with every tag on a node and on a relationship
- [ ] T013 [P] [US1] Write `tests/contract/load_typed.rs` and add `mod load_typed;` to `tests/contract/main.rs`: load each fixture and assert `valueType` of each property is DATE, ZONED TIME, LOCAL TIME, ZONED DATETIME, LOCAL DATETIME, DURATION, POINT, UUID and the values equal the text (scenario 1); the datetime keeps zone name and offset (scenario 4); an unrecognized tag (`md`) and a wrong-case tag (`Date`) load as strings with the drop reported (scenario 2); a typed tag in the top-level record is validated and not stored, with the existing warning (scenario 3); a reload of an edited file overwrites a typed value, and a plain string overwrites a typed one

### Implementation for User Story 1

- [ ] T014 [US1] In `src/values.rs` `convert_value`, for a `tagged_string` whose tag `Tag::from_name` recognizes: validate the content and return `PropValue::Typed` with the string's span, or push `value.tag-content` at that span naming the tag and the fault and return `None`; unrecognized tags keep the existing drop-and-report path and the `tags` setting (ADR-011 sections 2.2 and 2.3)
- [ ] T015 [US1] In `src/load.rs` leave typed values out of `props_value` (they are not written with `SET n = r.props`) and collect, after ids are known, a list of typed writes (element kind, internal id, key, typed value) for created nodes, matched-and-updated nodes, and relationships
- [ ] T016 [US1] In `src/load.rs` make the relationship-creation statement return the new relationship ids (as the node statement does) so typed relationship properties can be written by id
- [ ] T017 [US1] In `src/load.rs` add `write_typed`: group the collected writes by (element kind, key, tag, shape), where shape is the point's dimension; run one statement per group in batches of the existing batch size inside the load's transaction: `... SET n.` + a cypher name of the key (use `names::cypher_name`) + ` = date(r.v)` and the matching constructor for each tag, `point({x: r.x, y: r.y, srid: r.srid})` with `z: r.z` for 3D, by internal id (research R2); call it after the basic properties of each batch so the file's value wins on update
- [ ] T018 [US1] Run the T013 tests and the whole suite; confirm the failures from T013 now pass and nothing else changed

**Checkpoint**: typed values load; the library stores the right types.

---

## Phase 4: User Story 2 - A bad value is rejected with a position (Priority: P1)

**Goal**: every bad form is a located `value.tag-content`, found by `check` without a database; a database rejection is reported at the same place.

**Independent Test**: one invalid fixture per form, each with its code, line and column; the database unchanged.

### Tests for User Story 2 (write first, confirm they fail)

- [ ] T019 [P] [US2] Create invalid fixtures `tests/fixtures/invalid/tag-content-date-month.gram`, `-date-day-common-year.gram`, `-date-week.gram`, `-date-basic.gram`, `-time-no-offset.gram`, `-time-no-seconds.gram`, `-time-second-60.gram`, `-localtime-offset.gram`, `-datetime-bare-date.gram`, `-datetime-no-offset.gram`, `-duration-weeks.gram`, `-duration-leading-minus.gram`, `-duration-empty.gram`, `-duration-lowercase.gram`, `-duration-fraction-not-seconds.gram`, `-fraction-ten-digits.gram`, `-point-no-srid.gram`, `-point-bad-srid.gram`, `-point-dimension.gram`, `-point-exponent.gram`, `-point-latitude.gram`, `-uuid-braces.gram`, `-uuid-no-hyphens.gram`, `-empty-content.gram`, each with its `.expect` line `value.tag-content <line> <column> <offset>` at the tagged string
- [ ] T020 [P] [US2] Write a test in `tests/contract/load_errors.rs` that a file with several bad typed values reports every one with its position, up to `max_diagnostics`, with the total (scenario 4), and that a failed load leaves a database holding known content unchanged
- [ ] T021 [P] [US2] Write a test in `tests/contract/load_errors.rs` for scenario 3: a `datetime` with an unknown zone name and one whose offset is not the zone's pass `check` with no error, and fail the load with `value.tag-content` at the tagged string's position, with the database unchanged
- [ ] T022 [P] [US2] Write a test in `tests/contract/check.rs` that `check` reports `value.tag-content` for the T019 fixtures without a database (the existing `check_agrees_with_load_on_every_invalid_fixture` picks up the new fixtures; confirm it does)

### Implementation for User Story 2

- [ ] T023 [US2] In `src/load.rs` `write_typed`, map a database error from a typed statement to `value.tag-content` at the typed value's span (the message carries the database's text); the load reports it, rolls back, and returns the failure like other load errors (ADR-011 section 2.3)
- [ ] T024 [US2] Confirm `src/render.rs` renders `value.tag-content` with the source line and marker without a change; add a rendering test with a tagged string in `tests/render.rs`
- [ ] T025 [US2] Run the T019 to T022 tests and the whole suite; fix any validator whose position or message differs from the fixtures

**Checkpoint**: User Stories 1 and 2 give a working, strict typed load. This is the MVP.

---

## Phase 5: User Story 3 - Typed values export as tagged strings and round-trip (Priority: P1)

**Goal**: export writes typed values as tagged strings; export then load gives equal values.

**Independent Test**: a database with every type, exported and reloaded, compares equal.

### Tests for User Story 3 (write first, confirm they fail)

- [ ] T026 [P] [US3] Write `tests/contract/export_typed.rs` and add `mod export_typed;` to `tests/contract/main.rs`: build a database with Cypher constructors for each type (a date, a zoned time with nanoseconds, a local time, a zoned datetime with a zone name, a local datetime, a negative and a mixed-sign duration, a 2D geographic point, a 3D Cartesian point, a UUID); assert the exported text is exactly the expected tagged strings, byte-identical on a second export, and that the geographic point is written longitude first with its SRID (scenarios 1 and 3)
- [ ] T027 [P] [US3] In `tests/contract/export_typed.rs` add the round trip: export, load into a fresh database, compare every property's type and value with `dump_graph`
- [ ] T028 [P] [US3] Extend `tests/roundtrip/fixtures.rs` `FIXTURES` with the `typed-*` names so every valid typed fixture survives load, export and load

### Implementation for User Story 3

- [ ] T029 [US3] In `src/export.rs` add a column to the node and relationship queries that returns the temporal text: `[k IN keys(n) WHERE <valueType is a temporal or duration> | [k, toString(n[k])]]` (research R1); parse it into a map from key to text
- [ ] T030 [US3] In `src/export.rs` `scalar_text`, write `Value::Temporal` as `` tag`text` `` using that map (the tag from the value's kind), `Value::Uuid` with `relate::uuid_hyphenated`, and `Value::Point` as extended well-known text from its public fields with `float_text` (decimals without an exponent; `POINT Z` for 3D); remove date, point and uuid from the unwritable cases of `kind_name`/error text (ADR-011 sections 2.2 and 2.6)
- [ ] T031 [US3] Run the T026 to T028 tests and the whole suite

**Checkpoint**: typed values round-trip.

---

## Phase 6: User Story 4 - Lists of typed values (Priority: P2)

**Goal**: arrays of one recognized tag load and export as typed lists; mixes are errors.

**Independent Test**: load and export arrays of each tag; try the rejected mixes.

### Tests for User Story 4 (write first, confirm they fail)

- [ ] T032 [P] [US4] Create fixtures `tests/fixtures/valid/typed-lists.gram` (arrays of dates, durations, points, UUIDs), and invalid `tests/fixtures/invalid/array-mixed-tag-plain.gram` (a recognized tag and a plain string), `array-mixed-tag-two.gram` (two recognized tags), each `.expect` as `value.array-mixed <line> <column> <offset>` at the first element that differs
- [ ] T033 [P] [US4] Write tests in `tests/contract/load_typed.rs` and `export_typed.rs`: a typed list loads as `LIST<DATE ...>` and exports as the same array; an array of unrecognized tags loads as strings with the drops reported; a database list mixing a date and a datetime fails the export and is listed (scenarios 1 to 4); add `typed-lists` to the round-trip `FIXTURES`

### Implementation for User Story 4

- [ ] T034 [US4] In `src/values.rs` the `array` branch, treat an element tagged with a recognized tag as that tag's kind: all elements of one recognized tag give a `PropValue::List` of typed values; a recognized tag mixed with plain strings or another recognized tag is `value.array-mixed` at the first element that differs (ADR-011 section 2.4)
- [ ] T035 [US4] In `src/load.rs` `write_typed`, handle a list as shape `list`: `SET n.` + key + ` = [x IN r.v | date(x)]` with the matching constructor; points in a list use per-element maps
- [ ] T036 [US4] In `src/export.rs`, write a list of one typed kind as an array of tagged strings, using `[x IN n[k] | toString(x)]` text for temporals; a list that mixes kinds stays an `export.value` error (ADR-011 section 2.6)
- [ ] T037 [US4] Run the T032 and T033 tests and the whole suite

**Checkpoint**: typed lists work.

---

## Phase 7: User Story 5 - What cannot be written is reported, not skipped (Priority: P3)

**Goal**: vectors and the runtime values stay unwritable and listed.

**Independent Test**: export a database with a vector; load a `vector` tag.

### Tests for User Story 5

- [ ] T038 [P] [US5] Update `tests/contract/export_values.rs`: `every_value_with_no_gram_form_is_listed_before_any_output` and `relationship_values_are_checked_too` now use a date and a point as offenders, which are writable; replace them with a vector (`vector([1.0, 2.0], 2, FLOAT32)`) and a mixed temporal list, keeping the expected counts, and add a test that a vector is listed with its element and key (scenario 1)
- [ ] T039 [P] [US5] Add a test in `tests/contract/load_typed.rs` that a `vector` tag is unrecognized: the content loads as a string and the drop is reported (scenario 2)

### Implementation for User Story 5

- [ ] T040 [US5] In `src/export.rs` confirm vectors and byte arrays fall through to the existing `export.value` error with a message naming the kind (a vector and a byte array, not "temporal value"); adjust the message text if needed; run T038 and T039

**Checkpoint**: unwritable values are still loud.

---

## Phase 8: User Story 6 - Documentation and the fixer (Priority: P3)

**Goal**: the docs, contract, changelog and fixer behavior match.

**Independent Test**: each emitted code appears in the docs table and the contract; `fix` annotates a bad typed value.

- [ ] T041 [P] [US6] Write a test in `tests/fix.rs` that `fix` leaves a file with a bad typed tag unchanged and annotates the error with `// relate-gram: error[value.tag-content]: ...`, and that a second run does not duplicate the annotation (ADR-010 section 4)
- [ ] T042 [P] [US6] Update `docs/supported-gram.md`: add the eight recognized tags with their forms, the rewrite for each rejected form, `value.tag-content` and the extended `value.array-mixed` rows; state the three ISO 8601 extensions (a negative year, a bracketed zone name, a sign on each duration number)
- [ ] T043 [P] [US6] Merge `specs/002-typed-tags/contracts/diagnostics.md` into `specs/001-phase1-load-export/contracts/diagnostics.md` (add `value.tag-content`; mark `export.value` offenders changed) and note the new `PropValue::Typed` variant in `specs/001-phase1-load-export/contracts/library-api.md` (research R6)
- [ ] T044 [P] [US6] Update `README.md` ("What is accepted", the list of values with no gram form, "Known limits") and add to `CHANGELOG.md` under Unreleased: the eight tags, the new code, `PropValue::Typed`, and the migration note that a file whose string used one of these tags now loads typed values or fails (SC-006), with the rewrite to restore the old behavior
- [ ] T045 [US6] Update `CLAUDE.md` Scope bullet on tags to say recognized tags give typed values (ADR-011) and unrecognized ones are dropped

**Checkpoint**: documentation matches behavior.

---

## Phase 9: Polish and cross-cutting

- [ ] T046 [P] Add a typed-density case to `tests/perf.rs` (10,000 elements, every element carrying a date, a duration and a point) and to `benches/load_export.rs`; assert the load takes no more than three times the plain-string case (SC-004); run with `cargo test --release --test perf -- --ignored --nocapture` and record the numbers in `docs/` or the PR
- [ ] T047 Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `scripts/check-public-api.sh` and the whole `cargo test`; fix what fails (SC-005)
- [ ] T048 Run the steps of `specs/002-typed-tags/quickstart.md` by hand against a built binary and note any difference
- [ ] T049 Tick the completed tasks in this file; commit with the ADR-011 section cited

---

## Dependencies and execution order

- Phase 1 then Phase 2 (T002 before T010; T004 to T006 before T007 to T009; T010 and T011 last in Phase 2).
- US1 (Phase 3) needs Phase 2. US2 (Phase 4) needs US1's `write_typed` (T017) for T023. US3 (Phase 5) needs Phase 2 and the loader (US1) for its reload step. US4 needs US1 and US3. US5 needs US3's export changes. US6 needs the behaviors it documents.
- MVP: Phases 1 to 4 (a strict typed load with positions). Phase 5 completes the round trip, which Constitution IV expects before release.

## Parallel examples

- Phase 2: T004, T005, T006 together; then T007, T008, T009 together.
- US1 tests: T012 and T013 together.
- US2 tests: T019 to T022 together.
- US3 tests: T026 to T028 together.
- Documentation: T042 to T044 together.

## Implementation strategy

1. Land Phases 1 and 2 with their unit tests.
2. Land US1 and US2 as one change: a strict typed load.
3. Land US3 (export and round trip), then US4 (lists), then US5 and US6 with polish.
4. After each, run the full suite and the public-API check; cite ADR-011 in each commit.
