# Tasks: The Conformance Collection and its Rust Runner

**Input**: Design documents in `/specs/006-conformance-collection/` (spec.md, plan.md, research.md, data-model.md, quickstart.md); ADR-015.

**Tests**: Required. Constitution VI: every task that adds a runner piece starts from a failing test; every converted case is shown to fail when its expectation is made wrong before the test it replaces is removed.

**Format**: `- [ ] T### [P?] [US?] Description with file path`. `[P]` marks tasks on different files with no unfinished dependency.

## Baselines and Results

Recorded as tasks complete.

- `cargo test`, debug, warm build, before the conversion (T001): per-target test time 2.9 s in total (cli 1.26 s, roundtrip 0.53 s, embedding16 0.29 s, positions 0.16 s, corpus 0.17 s, contract 0.10 s, coverage 0.05 s, lib 0.04 s, the rest under 0.02 s).
- Position test 0.16 s; contract tests 0.10 s (148 tests); what the collection replaces is therefore about 0.3 s of test time, so the limit of SC-006 (twice that) is about 0.6 s plus the cost of reading files.
- Number of Rust tests before (T001): 256 `#[test]` in `tests/` (148 contract, 31 fix, 28 cli, 14 roundtrip, 9 render, 9 syntax, 7 corpus, 3 + 3 embedding16, 2 positions, 1 coverage) and 40 unit tests in `src/`.
- Runner wall time after the conversion (T038): _not yet measured_ (limit: twice the time of what it replaced, SC-006).
- Tests removed, by task (SC-007): _none yet_.

## Phase 1: Setup

- [x] T001 Record the baselines above in `specs/006-conformance-collection/tasks.md`: `cargo test` wall time, the time of `--test positions` and `--test contract`, and the count of `#[test]` per file (the list of research R10)
- [x] T002 Add `relateby-gram = "0.7"` to `[dev-dependencies]` in `Cargo.toml` and write a smoke test `tests/conformance_smoke.rs` that parses a one-line case file with `parse_gram_with_header` and reads its header and a labeled node; run `cargo test --test conformance_smoke` (compiles the dependency, ADR-015 section 2.10)
- [x] T003 [P] Create the skeleton under `conformance/`: `VERSION` (`0.1.0`), `GRAMMAR_VERSION` (the `tree-sitter-gram` version of `Cargo.toml` and the `gram-data` version installed), `vocabulary.gram` (the labels `Error`, `Warning`, `Counts`, `Applied`, `Remaining`, `Result`, `KnownFailure` and the capability names of ADR-015 section 2.6), `capabilities.gram` (the probes of research R8), `start.cypher` (`CREATE (:Existing {k: 1})-[:R]->(:Existing {k: 2});` from `tests/positions/main.rs`), an empty `known-failures.gram`, and `.gitattributes` with `conformance/**/*.txt -text` and `conformance/**/*.bin -text`
- [x] T004 [P] Add a step to `.github/workflows/ci.yml` that installs the `gram` tool (`cargo install gram-data --locked`) and sets `CONFORMANCE_REQUIRE_LINT=1` for the test step (research R2)

## Phase 2: Foundational (the shared runner module; blocks every story)

- [x] T005 [P] Write failing tests in `tests/conformance_unit.rs` (a target that includes `tests/conformance/mod.rs` with `#[path]`) for case reading: a case with `requires`, `settings` and `transaction`; each expectation label with its properties; a missing `requires`; an unknown key in `settings` or `Counts`; an unknown label; a file the kind does not use (ADR-015 section 2.2; spec edge cases)
- [x] T006 Implement `tests/conformance/mod.rs`, `collection.rs` (find `conformance/` from `CARGO_MANIFEST_DIR`, enumerate `<kind>/<id>`), `case.rs` (read `case.gram` with `relateby-gram`; files beside it) and `settings.rs` (lowerCamelCase keys to `LoadSettings`, `ExportSettings`, `FixOptions`, with defaults and the unknown-key failure) until T005 passes
- [ ] T007 Print `valueType()` and `toString()` for one property of every type the fixtures hold (integer, float, boolean, string, list of each, date, time, local time, datetime, local datetime, duration, point, UUID, float vectors of each kind) in a throwaway test run with `--nocapture`, and record the pairs under R3 in `specs/006-conformance-collection/research.md`
- [ ] T008 [P] Write failing tests in `tests/conformance_unit.rs` for the value table and graph lines: each pair of T007 and its expected-side counterpart reduce to one `gram_value::Value` text; tagged values meet at normal content (`10:30Z` and `10:30:00Z`); a node line and a relationship line have the form of research R4; two graphs equal ignoring a generated `@id`; graphs that differ in a label, a key or a value are not equal
- [ ] T009 Implement `tests/conformance/value.rs` (both sides of the table) and `graph.rs` (lines, the database dump through Cypher, the multiset comparison, the generated-identifier rule) until T008 passes
- [ ] T010 [P] Write failing tests in `tests/conformance_unit.rs` for the expected-graph rules: a path, a forward reference, an annotation, a map value, a repeated definition, a relationship end that is not defined at top level, a subject pattern, and a pattern with more than two elements each fail with a message naming the rule; a valid file passes (ADR-015 section 2.3; research R5)
- [ ] T011 Implement `tests/conformance/expected.rs` (read `expected.gram` into node and relationship lines under the rules) until T010 passes
- [ ] T012 Re-export `typed::elements::{widen, narrow}` in `__test_support` in `src/lib.rs` and use `widen` in `tests/conformance/value.rs` for float vector elements (research R6); run `scripts/check-public-api.sh`
- [ ] T013 [P] Write failing tests in `tests/conformance_unit.rs` for diagnostics and counts matching: errors and warnings compared exhaustively by code, line, column and offset (a diagnostic with no position by code); an extra, a missing and a different diagnostic each fail naming the part; `Counts` compares exactly the keys it lists; message wording and related positions are not compared (ADR-015 section 2.4)
- [ ] T014 Implement `tests/conformance/expect.rs` (the matching of T013) until it passes

**Checkpoint**: `cargo test --test conformance_unit` passes; no case exists yet.

## Phase 3: User Story 1 - Every rejected file is a case (Priority: P1) 🎯 MVP

**Goal**: the 71 invalid fixtures are `reject` cases and the runner reproduces their diagnostics.

**Independent Test**: `cargo test --test conformance reject`.

- [ ] T015 [US1] Write a throwaway conversion script in the scratchpad that turns each `tests/fixtures/invalid/<name>.gram` and `<name>.expect` into `conformance/reject/<name>/` with `input.txt` (byte-identical copy; `input.bin` for the invalid-UTF-8 fixture) and a `case.gram` holding `requires: []`, a doc comment from the fixture's own comment line, and one `Error` node per `.expect` row (spec User Story 1, scenarios 1 and 5); run it and check every `input.txt` is byte-identical with `cmp`
- [ ] T016 [US1] Write failing tests in `tests/conformance_unit.rs` that build in-memory `reject` cases with a wrong code, line, column, offset, an extra error and a missing error, and require the runner's `reject` function to fail each naming the part; and one correct case that passes (spec scenario 4)
- [ ] T017 [US1] Implement `tests/conformance/reject.rs`: load the text into a database made from the case's or the standard `start.cypher`, compare diagnostics (T014), require the starting graph unchanged by the graph dump (T009), and run `relate_gram::check` on the text and compare the same errors (ADR-015 section 2.2); add `tests/conformance.rs` that runs every case of `conformance/reject/` and prints a result line per case
- [ ] T018 [US1] Run `cargo test --test conformance`; for each failing case decide whether the case or the library is wrong and fix the case (the library does not change, FR-012); make a deliberately wrong `Error` row in one case and confirm it fails, then restore it
- [ ] T019 [US1] Add `conformance/**/*.txt` BOM and CRLF checks: an `input.txt` with a byte order mark and one with CRLF in the collection survive `git checkout` unchanged (verify with `git ls-files --eol` and a fresh clone into the scratchpad) (spec scenario 6)
- [ ] T020 [US1] Remove the tests the reject cases replace, naming each in `specs/006-conformance-collection/tasks.md` Results: `tests/positions/main.rs`, `check_agrees_with_load_on_every_invalid_fixture` and `check_reports_every_bad_typed_value_without_a_database` in `tests/contract/check.rs`, and any other test that iterates `tests/fixtures/invalid/`; then delete `tests/fixtures/invalid/` if nothing else reads it

**Checkpoint**: User Story 1 passes on its own; the 71 rejections are pinned by the collection.

## Phase 4: User Story 2 - Every valid file is a case with an expected graph (Priority: P1)

**Goal**: the 25 valid fixtures and 4 reload fixtures are `load` cases with expected graphs.

**Independent Test**: `cargo test --test conformance load`.

- [ ] T021 [US2] Write failing tests in `tests/conformance_unit.rs` for the `load` runner: an in-memory case whose `expected.gram` omits a property, changes a value, adds a node, or whose `Counts` or `Warning` differs fails naming the part; a correct case passes; a case with `start.cypher` and an identity property setting passes with no `@id` expected (spec User Story 2 scenarios 2, 4, 6)
- [ ] T022 [US2] Implement `tests/conformance/load.rs`: apply `settings`, run `start.cypher` (statements ending in a semicolon at the end of a line, in file order, no parameters), load `input.txt`, compare the warnings (T014) and the counts, dump the graph (T009) and compare it with `expected.gram` (T011), handle `transaction: true`; extend `tests/conformance.rs` to run `conformance/load/`
- [ ] T023 [US2] Write a throwaway script in the scratchpad that loads each valid fixture and reload fixture into a fresh database, dumps the graph through Cypher in the form of research R4 as gram (`expected.gram`), and writes `conformance/load/<name>/` with `input.txt`, `case.gram` (settings, `Warning` nodes, `Counts`) and, for the reload fixtures, a `start.cypher` that creates the earlier state; run it
- [ ] T024 [US2] Review every generated `expected.gram` against the assertions of the tests it replaces (`load_basic`, `load_values`, `load_typed`, `load_embedding`, `load_identity`, `identity_property` and `reload` in `tests/contract/`), correcting any expectation that merely records what the library does without the old test asserting it, and list the reviewed tests in Results (research R9)
- [ ] T025 [US2] Cover each Relate type that an expected graph can hold (spec scenario 7): list the types and the case that holds each in `conformance/README.md`; add a small `load` case for any type no fixture covers
- [ ] T026 [US2] Run `cargo test --test conformance`; confirm each load case fails when one property of its `expected.gram` is changed, then restore it
- [ ] T027 [US2] Remove the contract tests that a load case fully covers, naming each in Results, and delete the valid and reload fixtures that no remaining test reads

**Checkpoint**: User Stories 1 and 2 pass; every fixture is a case.

## Phase 5: User Story 3 - Exports are cases (Priority: P2)

**Goal**: golden export strings are `export` cases and exports of loaded graphs are checked.

**Independent Test**: `cargo test --test conformance export`.

- [ ] T028 [US3] Write failing tests in `tests/conformance_unit.rs` for the `export` runner: byte-for-byte text, a changed byte fails, an expected failure compared by code as a multiset whatever element is named, `transaction: true` giving the refusal code, a `load` case with `exported.txt` whose text loads back to an equal graph ignoring generated identifiers (spec User Story 3)
- [ ] T029 [US3] Implement `tests/conformance/export.rs` and the `exported.txt` step of `load.rs`; extend `tests/conformance.rs` to run `conformance/export/`
- [ ] T030 [US3] Convert the golden export strings of `tests/contract/export_basic.rs`, `export_values.rs`, `export_typed.rs`, `export_identity.rs` and `export_embedding.rs` to `export` cases (a `start.cypher` that builds the database, the settings, `exported.txt`) and the exports of the load cases to `exported.txt`; leave the tests of research R10 that stay in Rust; list what was converted and what stays in Results
- [ ] T031 [US3] Run `cargo test --test conformance`; change one byte of one `exported.txt` and confirm the case fails, then restore it
- [ ] T032 [US3] Remove the export tests the cases replace, naming each in Results

## Phase 6: User Story 4 - The fixer's examples are cases (Priority: P2)

**Goal**: fixer examples are `fix` cases.

**Independent Test**: `cargo test --test conformance fix`.

- [ ] T033 [US4] Write failing tests in `tests/conformance_unit.rs` for the `fix` runner: output text byte for byte, `Applied` by code and count, `Remaining` by code, line, column and offset in the output, the options from `settings`, a fix whose option is absent leaving the text as it was, `annotate` markers part of `fixed.txt` (spec User Story 4)
- [ ] T034 [US4] Implement `tests/conformance/fix.rs` and extend `tests/conformance.rs` to run `conformance/fix/`
- [ ] T035 [US4] Convert the input-to-output examples of `tests/fix.rs` to `fix` cases, leaving idempotence, `FixFailure`, round and annotation counters and the other checks of research R10 in Rust; list both groups in Results; run `cargo test --test conformance`; change one `Applied` count and confirm the case fails, then restore it; remove the tests the cases replace, naming each in Results

## Phase 7: User Story 5 - Capabilities, known failures and results (Priority: P2)

**Goal**: probes, `!`, two passes, not-applicable, known failures and the results file.

**Independent Test**: `cargo test --test conformance` and `--test conformance16`.

- [ ] T036 [US5] Write failing tests in `tests/conformance_unit.rs` for capabilities and results: a case whose requirement is not met is `not-applicable` and is not run; `!` inverts; a declared capabilities file that adds a capability is refused and one that removes with a reason is accepted; a known failure that fails is `skipped` with its reason and one that passes fails the run; the results file has one `Result` per case with `case`, `status` and `version`; an operation the implementation does not offer makes its cases `not-applicable` (spec User Story 5)
- [ ] T037 [US5] Implement `tests/conformance/capabilities.rs` (probes of research R8, the declared file) and `results.rs` (statuses, known failures from `conformance/known-failures.gram`, the results file under `target/conformance/`) until T036 passes
- [ ] T038 [US5] Add `tests/conformance16.rs` that sets `RELATE_PREVIEW=float16-coordinate-types` in a `Once` before the first connection and runs the same module; add cases that need the flag (`requires: ["vector-16-bit"]`: loads of the 16-bit embedding fixtures) and cases that need it absent (`requires: ["!vector-16-bit"]`: the load that stops before any write, from `tests/embedding16_off.rs`); run both targets and confirm that together the two results files cover every case (spec scenario 3)

## Phase 8: User Story 6 - The collection is checked, and the replaced tests are gone (Priority: P3)

**Goal**: the checks, the removal of what the collection replaces, and the documents.

**Independent Test**: `CONFORMANCE_REQUIRE_LINT=1 cargo test --test conformance_checks`.

- [ ] T039 [US6] Write failing tests in `tests/conformance_checks.rs` that each fault fails the check: a lint error in a `case.gram`, a label or capability not in `vocabulary.gram`, a missing `VERSION` or `GRAMMAR_VERSION` or a version not of the form `MAJOR.MINOR.PATCH`, a duplicate case identifier in one kind, a diagnostic code with no case and no listed reason, a missing `.gitattributes` rule; use temporary copies of the collection for the faults
- [ ] T040 [US6] Implement `tests/conformance_checks.rs`: run `gram lint --strict` as a subprocess over `case.gram` and `expected.gram` (skip with install instructions when the tool is absent unless `CONFORMANCE_REQUIRE_LINT=1`), the vocabulary check, the version files, identifier uniqueness, the `.gitattributes` rule, and the coverage of `relate_gram::diagnostics::code::ALL` with exceptions read from `conformance/README.md` (research R11) until T039 passes
- [ ] T041 [US6] Remove `tests/coverage.rs` (replaced by the coverage check) and record it in Results
- [ ] T042 [US6] Write `conformance/README.md`: the case form, the value table with the pairs of T007, the vocabulary, the capability probes, how to run both passes and the checks, the list of tests that stay in Rust with their reasons, and the exceptions of the coverage check (FR-013; ADR-015 section 2.8)
- [ ] T043 [P] [US6] Point `CLAUDE.md` (Build and test), `docs/quickstart.md` and `specs/001-phase1-load-export/` where it names the removed fixtures and tests at `conformance/README.md`, and update the fixture paths that moved

## Phase 9: Polish and Cross-Cutting

- [ ] T044 Measure the runner (`cargo test --test conformance` and `--test conformance16`) against the baseline of T001 and record both under Baselines (SC-006); if it exceeds twice the replaced tests, find the cost (the database per case, the dump) and reduce it
- [ ] T045 Run the five scenarios of `specs/006-conformance-collection/quickstart.md` and note any difference under Results
- [ ] T046 Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `scripts/check-public-api.sh`, `cargo test`, and the CI lint step locally with the `gram` tool; confirm `Cargo.toml` differs only by the dev-dependency (SC-006)
- [ ] T047 Tick every completed task above, fill Baselines and Results, bump `conformance/VERSION` if a case changed after 0.1.0, and cite ADR-015 in the commit messages of the branch

## Dependencies and Order

- Phase 1, then Phase 2 (T005 before T006, T008 before T009, T010 before T011, T013 before T014).
- US1 needs Phase 2. US2 needs Phase 2 and the reject runner's file reading (T017). US3 needs US2's graph dump. US4 needs Phase 2 only. US5 needs US1 and US2 to have cases to report on. US6 needs all other stories.
- Within a story: failing tests, then conversion, then runner, then run, then remove the replaced tests.

## Parallel Opportunities

- T003 and T004. T005, T008, T010 and T013 are tests on one file but independent pieces and can be written together. T043 with T042.
- After Phase 2, US1 and US4 touch different kinds and can proceed in parallel.

## Implementation Strategy

MVP is Phase 1, Phase 2 and User Story 1: the 71 rejections run from the collection. User Story 2 adds the valid fixtures, the part with the most writing. User Stories 3 and 4 follow, then capabilities and results, then the checks and documents. The replaced tests are removed only after their case has been seen to fail with a wrong expectation.
