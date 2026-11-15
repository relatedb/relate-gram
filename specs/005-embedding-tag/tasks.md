# Tasks: The embedding Tag

**Input**: Design documents in `/specs/005-embedding-tag/` (spec.md, plan.md, research.md, data-model.md, quickstart.md); ADR-014.

**Tests**: Required. Constitution VI: every test task is written and seen to fail before the code that satisfies it.

**Format**: `- [ ] T### [P?] [US?] Description with file path`. `[P]` marks tasks on different files with no unfinished dependency.

## Baselines and Results

Recorded as tasks complete.

- Typed 10,000-element load before changes (T001): 373 ms (run on this machine: 344 to 377 ms across earlier runs; ceiling 1.2 times, SC-005).
- 1,000 vectors of dimension 1,536, load and export, on the finished feature (T030): load 281 to 296 ms, export 26 to 27 ms over three release runs (ceiling for later changes: 1.2 times, so load 355 ms and export 32 ms; SC-006).
- Typed 10,000-element load after the feature (T030): 340 ms, within the ceiling (SC-005).

## Phase 1: Setup

- [x] T001 Run `cargo test` and `cargo test --release --test perf -- --ignored --nocapture` on the branch as it stands (gram-value 0.2 bump, no feature code) and record the typed 10,000-element load time under Baselines in `specs/005-embedding-tag/tasks.md`
- [x] T002 [P] Copy the `gram-value` 0.2.0 corpus cases `tag-embedding-ref-f32`, `-ref-f64`, `-ref-f16`, `-bf16`, `-nonnormal`, `-dim-1`, `-dtype-i32`, `-dtype-case`, `-b64-bad-char` and `-empty` from `/Users/abk/Developer/gram-data/gram-value/conformance/tags.gram` into fixture files under `tests/fixtures/valid/embedding-*.gram` and `tests/fixtures/invalid/embedding-*.gram`, each with a comment naming the case id and `gram-value` 0.2.0 (plan R8)
- [x] T003 [P] Add fixture helpers to `tests/support/mod.rs` that build an `embedding` tagged string of a given dtype, dimension and element list (including a NaN and a dimension of 4097) with the SafeTensors layout of `gram-value` ADR-008 section 2.2, for the Relate-specific limits that the corpus does not cover (plan R8)

## Phase 2: Foundational (blocks every story)

- [x] T004 Write failing unit tests in `src/typed/elements.rs` for `widen(dtype, bytes) -> Vec<f64>` and `narrow(dtype, &[f64]) -> Vec<u8>`: `F32` `[1.0, -2.0]`, `F64`, `BF16` `[1.0, -2.0]` (`0x3f80`, `0xc000`), `F16` `[1.0, 1.5, 2.0, 3.0, 6.0]` as in the corpus, subnormals and negative zero for both 16-bit formats
- [x] T005 Implement `src/typed/elements.rs` and add `mod elements;` to `src/typed/mod.rs`: `F32` and `F64` by `from_le_bytes`/`to_le_bytes`, `BF16` as the top 16 bits of an f32, `F16` by bit fields with round-to-nearest-even; no new dependency (FR-009, plan R6)
- [x] T006 Write failing unit tests in `src/typed/mod.rs` and `src/typed/profile.rs`: `Tag::from_name("embedding")` is `Some(Tag::Embedding)`, `Tag::ALL` has nine, `validate` returns the normal content for the reference F32 case and for the non-normal case, a dimension of 4097 is an error stating Relate's limit of 4096 and that the tag allows more, a NaN at index 2 is an error naming index 2, and an embedding that is both too large and non-finite reports the dimension first (FR-001, FR-004)
- [x] T007 Add `Tag::Embedding` (name `embedding`) to `src/typed/mod.rs`, extend `Tag::ALL` to nine, and add the `Typed::Embedding` arm to `profile::check` in `src/typed/profile.rs`: dimension above 4096 first, then the first non-finite element via `elements::widen` (research R1)

**Checkpoint**: `cargo test --lib` passes; `check` of a fixture with a good embedding reports no error.

## Phase 3: User Story 1 - A file with embeddings loads them as vectors (Priority: P1) 🎯 MVP

**Goal**: F32 and F64 embeddings load as `FLOAT32` and `FLOAT64` vectors.

**Independent Test**: `cargo test --test contract load_embedding`.

- [x] T008 [US1] Write failing tests in `tests/contract/load_embedding.rs` (add the `mod` line to `tests/contract/main.rs`): an `F32` embedding loads as a `Value::Vector` of kind `Float32` with the payload's elements, an `F64` one as `Float64`, on a node and on a relationship; the non-normal case loads to the same vector as the reference case; loading `embedding` content under the tag policy `warn` reports no `value.tag-dropped` for it, and `md` is still dropped (User Story 1 scenarios 1 to 6)
- [x] T009 [US1] Give `TypedGroup` in `src/load.rs` a `dtype: Option<EmbeddingDtype>` set for embeddings by `gram_value::tags::interpret_tag("embedding", content)`, return a `Value::List` of floats from `elements::widen` in `typed_param`, and build `vector({var}, size({var}), KIND)` with a literal `FLOAT32` or `FLOAT64` in `typed_expr` for an embedding (FR-002, FR-003, research R3)
- [x] T010 [US1] Make `typed_tag` and `group_spans` in `src/load.rs` handle `Tag::Embedding` (a scalar only; list handling is unchanged for the other tags) and run `cargo test --test contract load_embedding` to green

**Checkpoint**: User Story 1 passes on its own.

## Phase 4: User Story 2 - A database with float vectors exports and round-trips (Priority: P1)

**Goal**: float vectors export as `embedding`; integer vectors are refused with a reason.

**Independent Test**: `cargo test --test contract export_embedding` and `cargo test --test roundtrip embedding`.

- [x] T011 [US2] Update the two tests in `tests/contract/export_values.rs` that use a property named `embedding` holding a `FLOAT32` vector to assert refusal (lines near 136 and 163) so they use an `INTEGER16` vector instead, keeping them as refusal tests (research R9)
- [x] T012 [US2] Write failing tests in `tests/contract/export_embedding.rs` (add the `mod` line): a `FLOAT32` and a `FLOAT64` vector export as `embedding` with the normal content of `gram_value::tags::Embedding::to_value`, the reference-writer content round-trips byte for byte, an integer vector is an `export.value` error naming the element and key and saying the tag carries float vectors only, a database with integer and float vectors lists every integer one and writes nothing, and a test creates one vector of each of the eight kinds and checks the kind-name mapping (research R5; FR-007, FR-008)
- [x] T013 [P] [US2] Write failing round-trip tests in `tests/roundtrip/embedding.rs` (add the `mod` line to `tests/roundtrip/main.rs`): for `F32` and `F64`, dimensions 1, 3, 384 and 4096, with negative zero, subnormal and extreme finite elements, export then load into an empty database gives equal kind, dimension and elements (SC-001)
- [x] T014 [US2] Add the `Value::Vector { coord, values }` arm to the property-value mapping in `src/export.rs`: read the kind from `format!("{coord:?}")`, narrow each `Value::Float` with `elements::narrow`, build `Embedding::new(dtype, values.len(), bytes)` and return `.to_value()`; an integer kind or an unknown name is the error `a vector of kind X has no gram form; the embedding tag carries float vectors only`; remove `Vector` from the unwritable list (research R5; ADR-014 section 2.3)
- [x] T015 [US2] Run `cargo test --test contract` and `cargo test --test roundtrip` to green, and confirm the golden strings of every other type are unchanged (FR-012, User Story 2 scenario 7)

**Checkpoint**: User Stories 1 and 2 pass; a float-vector database round-trips.

## Phase 5: User Story 3 - A bad or unstorable embedding is a located error (Priority: P1)

**Goal**: `check` and `load` report each rejection with a code, line and column.

**Independent Test**: `cargo test --test contract check` and `load_embedding` rejection tests.

- [x] T016 [US3] Write invalid fixtures with `.expect` files under `tests/fixtures/invalid/`: `embedding-bad-b64`, `embedding-dtype-i32` (code `value.tag-content`), `embedding-dtype-case`, `embedding-dim-4097`, `embedding-nan`, `embedding-array`, plus one fixture where an array mixes an `embedding` with a string (expected `value.array-mixed`); line and column are those of the tagged string, or of the array for `embedding-array` (spec User Story 3 scenarios 1 to 5)
- [x] T017 [US3] Write failing tests in `tests/contract/check.rs` and `load_embedding.rs`: every fixture of T016 gives its `.expect` code, line and column from `check` and from `load`; dimension 4096 passes; a rejected embedding in a file stores nothing; `fix` leaves the file unchanged and annotates the error (add the case to `tests/fix.rs`) (scenarios 6 and 7; FR-010)
- [x] T018 [US3] In `src/values.rs`, in the `array` lowering, before converting the elements, report one `value.tag-content` error at the array node ("an array of embeddings has no Relate type; write one property per vector") when every named child is a tagged string whose tag is `embedding`, and return `None`; leave the mixed case to the existing homogeneity check (FR-005, research R2)
- [x] T019 [US3] Confirm no fix rule in `src/fix.rs` matches a `value.tag-content` for `embedding` and run the tests of T017 to green

**Checkpoint**: User Stories 1 to 3 pass; the strictness rule holds for the new tag.

## Phase 6: User Story 4 - 16-bit embeddings load only when Relate allows them (Priority: P2)

**Goal**: F16 and BF16 load with the preview flag; without it the load stops before writing and reports each value.

**Independent Test**: `cargo test --test embedding16` and `cargo test --test embedding16_off`.

- [x] T020 [US4] Write failing tests in a new `tests/embedding16.rs` that set `RELATE_PREVIEW=float16-coordinate-types` through a `Once` before the first `fresh_db()` (research R7): `F16` and `BF16` embeddings load as `Float16` and `BFloat16` vectors and export to the original normal content (User Story 4 scenario 1)
- [x] T021 [US4] Add to `tests/embedding16.rs` the exhaustive test: for every finite bit pattern of `F16` and of `BF16` (65,536 each, non-finite skipped), build the payload, widen, store in a Relate vector in batches, read back, narrow, and assert identical bytes, including subnormals and negative zero (FR-009, SC-003)
- [x] T022 [US4] Write failing tests in a new `tests/embedding16_off.rs` that never set the flag: a file with two `F16` and one `BF16` embeddings fails with nothing written (the database is empty afterwards) and one `value.tag-content` error at each of the three values, each naming the flag and `RELATE_PREVIEW=float16-coordinate-types`; a file with only `F32` and `F64` loads; `check` of the 16-bit file passes (scenarios 2 to 4)
- [x] T023 [US4] Make the 16-bit kinds work in `typed_expr` in `src/load.rs` (`FLOAT16`, `BFLOAT16` literals) and widen/narrow in `src/typed/elements.rs`; run `cargo test --test embedding16` to green
- [x] T024 [US4] Add the probe to `load::write` in `src/load.rs`: before `write_plan`, collect the embeddings whose dtype is `F16` or `BF16`; if any, run `RETURN vector([0.0], 1, FLOAT16)`, and on error return one `value.tag-content` diagnostic per such value with the message of research R4, writing nothing; no probe when none is present; the loader never sets the flag (FR-006); run `cargo test --test embedding16_off` to green

**Checkpoint**: all four stories pass; both 16-bit targets are green.

## Phase 7: User Story 5 - The change is documented (Priority: P2)

**Goal**: the documents and earlier decisions say what ADR-014 section 2.6 lists.

**Independent Test**: search the documents; read the changelog.

- [x] T025 [P] [US5] Update `docs/supported-gram.md`: nine tags and the `embedding` form, drop `vector` from the example of an unrecognized tag, replace "Vectors cannot be written in gram, in any form" with the float-vector rule and the integer refusal, add the four rejections (dimension above 4096, a non-finite element, an array of embeddings, a 16-bit dtype without the flag) to the rejected table, and say that `check` cannot know the preview flag (User Story 5 scenario 1)
- [x] T026 [P] [US5] Update `CLAUDE.md`: change "Vectors cannot be written in gram" in the Scope section to the float vectors that `embedding` carries
- [x] T027 [P] [US5] Add the amendment notes to the headers of `design/adr/ADR-011-typed-tags-and-empty-lists.md` (2.8 and 3.6 stand for integer vectors only; the passages at its opening paragraph, 2.1, 2.9 and the vector consequence) and a row for `embedding` ("no fix") to the fix table of `design/adr/ADR-010-fixing-rejected-files.md`; resolve question 7.3 for vectors in `design/rfc/RFC-002-property-value-mapping.md`
- [x] T028 [P] [US5] Add the `Tag::Embedding` variant and the new behavior to `specs/001-phase1-load-export/contracts/library-api.md`, and an amendment line to `specs/003-gram-value-tags/spec.md` header
- [x] T029 [US5] Add the Unreleased entries to `CHANGELOG.md`: the tag, a file that used `embedding` as a free tag now loads vectors, the 16-bit condition, integer vectors still refused, and `Tag` gaining a variant (library users matching exhaustively must add an arm)

## Phase 8: Polish and Cross-Cutting

- [x] T030 Add an `#[ignore]` perf case to `tests/perf.rs` that loads and exports 1,000 `FLOAT32` vectors of dimension 1,536, run `cargo test --release --test perf -- --ignored --nocapture`, record the figure under Baselines (SC-006) and re-run the typed 10,000-element load against the ceiling of T001 (SC-005)
- [x] T031 Run the five scenarios of `specs/005-embedding-tag/quickstart.md` against the built `relate-gram` and note any difference under Results
- [x] T032 Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `scripts/check-public-api.sh`, `cargo test` and confirm `cargo tree` differs from the main branch only by the `gram-value` bump (SC-007)
- [x] T033 Tick every completed task above, fill Baselines and Results, and cite ADR-014 in the commit messages of the branch

## Dependencies and Order

- Phase 1 first. Phase 2 blocks all stories (T004 before T005, T006 before T007).
- US1 needs Phase 2. US2 needs Phase 2 and `elements::narrow`; it is independent of US1 except that its round-trip tests load. US3 needs Phase 2 and T018 is independent of US1 and US2. US4 needs US1 and US2. US5 can start once the behavior is settled (after US4) but its tasks are on different files and run in parallel.
- Within a story: tests, then code, then green.

## Parallel Opportunities

- T002 and T003. T013 with T012. T025 to T028 together.
- After Phase 2, US1, US2 and US3 touch different parts (`load.rs` typed step, `export.rs`, `values.rs`) and can proceed in parallel by different hands; in one session they run in priority order.

## Implementation Strategy

MVP is Phase 1, Phase 2 and User Story 1: a file with embeddings loads. User Stories 2 and 3 complete the strict round trip and are needed before release. User Story 4 adds the 16-bit kinds, and User Story 5 and the polish phase finish the work for 0.3.0.

## Results

- All 33 tasks done. `cargo test`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `scripts/check-public-api.sh` pass; `Cargo.toml` and `Cargo.lock` differ from `main` only by the `gram-value` 0.2 bump (PR 10).
- Order of work: the failing-first step was seen for the tag and profile (T006), the load tests (T008), the export tests (T012), the rejection tests (T017) and the probe test (T022). The element-conversion unit tests (T004) were written in the same step as the code (T005), and the round-trip tests (T013), the flag-on 16-bit tests (T020, T021) and the perf case were written after the code they exercise and passed at once; the 16-bit statement needed no code beyond the literal kind keyword of T009.
- The exhaustive 16-bit test (T021) runs end to end (gram text, load, Relate's storage, export) for every finite pattern of `F16` and `BF16`, in 16 chunks of at most 4096 elements each, and compares the exported content with the input. Together with the unit test in `src/typed/elements.rs` it shows Relate rounds an exactly representable value to itself.
- Quickstart scenarios 1, 2 and 4 were run against the built command (`check`, `normalize`, a 16-bit file with and without `RELATE_PREVIEW`); scenario 3 is covered by `tests/contract/export_embedding.rs`.
- One decision made while implementing: the probe is the first statement of the load's transaction function, before the scan, so a refused 16-bit load changes nothing.
