---
description: "Task list for Phase 1 Gram Load and Export"
---

# Tasks: Phase 1 Gram Load and Export

**Input**: Design documents from `/specs/001-phase1-load-export/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/ (all present)

**Tests**: Required. Constitution VI (test-first, non-negotiable) demands contract, round-trip and error-position tests, so each phase writes its tests first and confirms they fail before the implementation tasks begin.

**Organization**: Grouped by user story. Each story's behavior is decided by the ADRs the spec names; commits and pull requests cite the ADR a task implements (Constitution I).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an incomplete task)
- **[Story]**: US1 to US5, from spec.md
- Paths are relative to the repository root

---

## Phase 1: Setup

**Purpose**: a building, empty crate with the pinned toolchain and the corpus submodule.

- [X] T001 Create `Cargo.toml`: package `relate-gram`, lib name `relate_gram`, edition 2021, `rust-version = "1.90"`; dependencies `relate` (git `https://github.com/neo4j-labs/cypherlite`, `rev = "84859b14"`, research R1; try `default-features = false` to drop the parquet snapshot codec, and keep it off if the crate builds and the spike probes still pass, recording the result in research R1), `tree-sitter = "0.25"`, `tree-sitter-gram = "=0.6.1"`, `thiserror`; dev-dependencies `insta`, `proptest`
- [X] T002 [P] Create `rust-toolchain.toml` pinning `1.96.0` with the `rustfmt` and `clippy` components (research R2)
- [X] T003 [P] Add the git submodule `external/tree-sitter-gram` at tag `v0.6.1` (corpus source only; ADR-007 §2.6)
- [X] T004 Create `src/lib.rs` with `#![forbid(unsafe_code)]` and empty module declarations for `diagnostics`, `settings`, `names`, `syntax`, `values`, `plan`, `load`, `export`; confirm `cargo build` succeeds
- [X] T005 [P] Create `.github/workflows/ci.yml`: checkout with submodules, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, and a manually triggered job that runs `cargo test --test perf -- --ignored`
- [X] T006 [P] Create `tests/fixtures/README.md` defining the fixture convention: a `.gram` file under `valid/` or `invalid/`, and for `invalid/` a sibling `.expect` file with one line per expected diagnostic, `code line column offset` (1-based line and column, byte offset)
- [X] T007 Create the test-target entry files `tests/contract/main.rs`, `tests/positions/main.rs` and `tests/roundtrip/main.rs`, each declaring `#[path = "../common/mod.rs"] mod common;` and one `mod` line per test file in its folder (every later task that adds a test file to one of these folders adds its `mod` line); create an empty `tests/common/mod.rs` so they compile (the helpers arrive in the support task below); `tests/syntax.rs` and `tests/corpus.rs` are single-file targets. Cargo builds only `tests/<name>.rs` and `tests/<name>/main.rs`, so `cargo test --test contract` needs this

**Checkpoint**: `cargo build` and `cargo test` run (no tests yet).

---

## Phase 2: Foundational (blocks every user story)

**Purpose**: diagnostics, settings, name quoting, the syntax layer and the shared test support.

- [X] T008 [P] Write unit tests in `src/diagnostics.rs` for `Span` (line, column and byte offset from a byte offset into text with LF and CRLF, multi-byte characters counted in bytes) and for the `Diagnostic` constructors; confirm they fail
- [X] T009 Implement `src/diagnostics.rs`: `Span`, `Severity`, `Diagnostic` (fields per data-model.md and contracts/library-api.md) and one `&'static str` constant per code in `contracts/diagnostics.md` (T008 passes)
- [X] T010 [P] Write tests in `src/settings.rs` for `LoadSettings` (`node_id_property`, `relationship_id_property`, `tags: TagPolicy`, `max_diagnostics`, and `file_name: Option<String>` so ADR-003 §2.1's file-name kind check can run), `ExportSettings`, `TagPolicy` and `IdentityMode` defaults as in contracts/library-api.md; confirm they fail
- [X] T011 Implement `src/settings.rs` (the preceding settings tests pass); update `contracts/library-api.md` and `data-model.md` to add `file_name`
- [X] T012 [P] Write unit tests in `src/names.rs` for: identifier text written as a bare integer (`-?(0|[1-9][0-9]*)`), bare symbol (`[A-Za-z_][0-9A-Za-z_.@-]*`) or backtick-quoted name, with `@`-leading text and `true` or `false` quoted and escapes for backtick, backslash, newline, carriage return and tab (ADR-002 §2.6); the same rule for labels, types and property keys (ADR-006 §2.5, ADR-008 §2.4); Cypher-name quoting that doubles a backtick; rejection of a NUL (research R6); confirm they fail
- [X] T013 Implement `src/names.rs` (T012 passes)
- [X] T014 [P] Create `tests/common/mod.rs`: open a fresh `Database::open(":memory:")` connection; helpers `load_str`, `export_str`, `assert_diag(&failure, code, line, column)`, and `dump_graph(&mut Connection)` returning nodes and relationships (labels, type, properties, endpoints by `@id` or position) in a comparable, order-independent form
- [X] T015 [P] Write tests in `tests/syntax.rs` (through a `#[doc(hidden)] pub mod __test_support` in `src/lib.rs` that re-exports `syntax::parse`): non-UTF-8 input gives `input.encoding` at the first bad byte offset; a leading byte order mark is skipped; CRLF and LF give correct columns; `(a {k: })`, `(a {xs: []})` and a raw newline in a quoted string give `parse.missing` or `parse.error` with the right position; all errors are collected and the cap and total apply (ADR-007 §2.2); confirm they fail
- [X] T016 Implement `src/syntax.rs`: parse bytes with `tree-sitter` and `tree-sitter-gram`; if the root `has_error`, collect ERROR and MISSING nodes as diagnostics; otherwise expose typed views over nodes (patterns, relationships, record, property, value kinds) with spans from the node (row + 1, byte column + 1, byte range) (T015 passes)

**Checkpoint**: foundation tests pass; stories can start.

---

## Phase 3: User Story 1 - Load a gram file into an in-memory database (Priority: P1) MVP

**Goal**: a valid gram file becomes the graph it describes, in an empty database.

**Independent Test**: load `tests/fixtures/valid/people.gram` into an empty database and compare nodes, labels, types, endpoints and properties with the file.

### Tests for User Story 1 (write first; they must fail)

- [X] T017 [US1] Create (before T018 to T020, which read them) valid fixtures under `tests/fixtures/valid/`: `people.gram` (labelled nodes, properties, typed directed relationships, a path, a left arrow, comments of both kinds), `values.gram` (every accepted value kind), `identity.gram` (identifier forms, references, anonymous elements), `empty.gram`, `record-only.gram`
- [X] T018 [P] [US1] Write `tests/contract/load_basic.rs`: Story 1 scenarios 1, 2 and 5 (ADR-001 §2.6, §2.9; ADR-007 §2.5): elements, creation order equal to first appearance (compare `id()` order), path equals separate relationships, mixed-direction path `(a)-[:R]->(b)<-[:S]-(c)`, empty and record-only files, comments ignored with no diagnostics, and a label and a relationship type containing a backtick, a space and `@` (written to Cypher as quoted names, research R6) loading and reading back unchanged
- [X] T019 [P] [US1] Write `tests/contract/load_values.rs`: Story 1 scenario 4 (ADR-006 §2.1, §2.2): integers including `i64::MIN` and `i64::MAX`, hexadecimal `0x1F` as 31, decimals as floats, booleans, strings in all four styles with escapes resolved per style, fenced content (starts after the line break following the fence, keeps its final newline and backslashes), bare symbols as strings, the empty string, a key with spaces, homogeneous arrays, tagged strings loaded as strings with one `value.tag-dropped` warning each
- [X] T020 [P] [US1] Write `tests/contract/load_identity.rs`: Story 1 scenario 3 (ADR-001 §2.1 to §2.6, §2.8; ADR-004): `@id` stored on identified elements (integer `42` stored as `"42"`, `` `a` `` equals `a`), none on anonymous ones, `load.anonymous` count, a node referenced before and after its definition, a referenced-only node gives an empty node and `id.reference-only`, relationship identifiers, no `@id` on anonymous relationships

### Implementation for User Story 1

- [X] T021 [US1] Implement the accepted-value mapping in `src/values.rs`: gram value nodes to `PropValue` per ADR-006 §2.1 and §2.2 (string escape resolution per quoting style, hex conversion, decimals, homogeneous arrays, tag extraction for inline and fenced tagged strings with the `value.tag-dropped` warning); the rejection branches come in US2
- [X] T022 [US1] Implement element extraction in `src/plan.rs`: nodes, relationships and paths from the typed views, labels in written order, the relationship type from its single label, direction from right and left arrows (ADR-008 §2.1), identifier normalization (backtick removal, integer as text), elements kept in first-appearance order
- [X] T023 [US1] Implement identifier resolution for valid files in `src/plan.rs` (ADR-001 §2.1 to §2.6): one shared identifier space, first appearance fixes the kind, definition versus reference, anonymous elements, referenced-only nodes with `id.reference-only`, the `RecordInfo` and the `LoadCounts`
- [X] T024 [US1] Implement `src/load.rs` for an empty database (matching against stored elements arrives in US3, so until then a file with identifiers loaded into a database that already holds the same `@id` values creates duplicates): begin a transaction, create nodes then relationships in consecutive-run batches of at most 5,000 rows with `UNWIND $rows` (research R5), labels and types written with `names.rs` Cypher-name quoting (R6), properties as parameter maps including `@id`, node ids returned in row order and mapped back to plan elements, `commit`, and `rollback` after any failed statement (SPIKE-001 §4.4)
- [X] T025 [US1] Implement the public `load` in `src/lib.rs` and the `LoadReport`, `LoadFailure` and `LoadCounts` types (contracts/library-api.md); wire syntax, plan and load together
- [X] T026 [P] [US1] Create `examples/load_and_query.rs`: opens an in-memory database, loads the file given as an argument, prints the diagnostics, counts of nodes and relationships, and, on failure, exits non-zero after printing that the database is unchanged (quickstart steps 1 and 2)

**Checkpoint**: Story 1 passes on its own; the crate loads valid files into an empty database.

---

## Phase 4: User Story 2 - Reject what does not map to Cypher, with a position (Priority: P1)

**Goal**: every unsupported or contradictory construct fails the load with a coded, located diagnostic, and the database is unchanged.

**Independent Test**: load each invalid fixture into a database holding known content; check the code, line, column and offset, and that the content is unchanged.

### Tests for User Story 2 (write first; they must fail)

- [X] T027 [P] [US2] Create invalid fixtures with `.expect` files for constructs and syntax: annotation, subject pattern, `::` label, an unsupported node kind, a MISSING node, a raw newline in a quoted string, non-UTF-8 input (`tests/fixtures/invalid/construct-*.gram`, `syntax-*.gram`)
- [X] T028 [P] [US2] Create invalid fixtures with `.expect` files for relationships (ADR-008): undirected, bidirectional, fat and wavy arrows, untyped definition (`rel-no-type.gram`, also used by the quickstart), bare `-->`, two labels (`tests/fixtures/invalid/rel-*.gram`)
- [X] T029 [P] [US2] Create invalid fixtures with `.expect` files for values and keys (ADR-006): range, measurement, nested map, octal, `9223372036854775808`, a decimal that overflows, a decimal that rounds to zero, mixed arrays (position of the first differing element), repeated key (with the related span), empty key, `"@k"` key, a tag under the error policy (`tests/fixtures/invalid/value-*.gram`, `key-*.gram`)
- [X] T030 [P] [US2] Create invalid fixtures with `.expect` files for identity and file (ADR-001, ADR-003): kind conflict, second definition (node and relationship), differing endpoints, a relationship identifier only referenced, record `kind` other than `property-graph`, a file name whose kind is not `property-graph` (`tests/fixtures/invalid/id-*.gram`, `file-*.gram`)
- [X] T031 [US2] Write `tests/positions/main.rs`: for every fixture, load into a database that already holds known content, assert each expected code, line, column and offset, and assert the database is unchanged; and assert that every fixture-testable error code in the Input, Relationships, Values and Keys, Identity (`id.kind-conflict`, `id.duplicate-definition`, `id.endpoints-differ`, `id.rel-never-defined`) and File (`file.kind`) groups of `contracts/diagnostics.md` has at least one fixture. Codes that cannot be fixtures are tested elsewhere: `match.*` and `id.property-missing` (T044, T045), the warnings `id.reference-only`, `load.anonymous`, `value.tag-dropped` and `file.record-not-stored` (T019, T020, T032), `load.in-transaction` (T032) and `load.database-error` (T032); `construct.unsupported` is covered by a unit test in `src/plan.rs` with a synthetic node kind
- [X] T032 [P] [US2] Write `tests/contract/load_errors.rs`: Story 2 scenarios 3 to 6: several errors reported together in source order, the cap and total, `TagPolicy::Error` listing every tag, the record-not-stored warning listing keys, `kind` checked and not listed, record values checked under ADR-006, `load.in-transaction` precondition, a Relate error mapped to `load.database-error` (for example a failure injected through a statement the test forces to fail), valid elements before an error are not written

### Implementation for User Story 2

- [X] T033 [US2] Add construct rejection in `src/plan.rs` and `src/syntax.rs`: annotations, subject patterns and pattern references, `::`, and any node kind not read (`construct.*`, ADR-007 §2.4); comments (`//` and `/** */`) stay ignored (§2.5)
- [X] T034 [US2] Add relationship rules in `src/plan.rs` (ADR-008 §2.2, §2.3): undirected, bidirectional and fat or wavy arrows (family from the first character of the arrow's text), no label, two or more labels, bare arrow, with the messages that say what to write
- [X] T035 [US2] Add rejection branches in `src/values.rs` (ADR-006 §2.1 to §2.4): range, measurement, nested map, octal, integer range, non-finite or underflowing decimals, mixed arrays, repeated keys with `related`, empty keys, `@` keys, and the `TagPolicy::Error` form
- [X] T036 [US2] Add identity errors in `src/plan.rs` (ADR-001 §2.2 to §2.4, §2.8): kind conflict, second definition, differing endpoints, relationship identifier never defined, each with `related` spans
- [X] T037 [US2] Add file kind and record handling in `src/plan.rs` (ADR-003): the `kind` check (record and `file_name`), the message "relate-gram loads only kind 'property-graph'; this file is kind '<kind>'", record values checked, and the `file.record-not-stored` warning
- [X] T038 [US2] Make `src/load.rs` and `src/lib.rs` stop on plan errors before touching the database; apply `max_diagnostics` with the total; return `load.in-transaction` when the connection is already in a transaction; map Relate errors to `load.database-error`
- [X] T039 [P] [US2] Write tests for the corpus reader in `tests/support/corpus_reader.rs`: parsing `tree-sitter test` corpus files (`====` headers, input, `---`, expected tree) from a small sample, including a case with several tests in one file; confirm they fail
- [X] T040 [US2] Implement the corpus reader in `tests/support/corpus_reader.rs` (the preceding tests pass), reading `external/tree-sitter-gram/test/corpus/*.txt`
- [X] T041 [US2] Author `tests/support/corpus_expectations.toml`: one entry per corpus case, either `accept` with the expected graph (node and relationship counts, labels, types, property keys and values) or `reject` with the expected diagnostic code and position; cases that use annotations, patterns, `::`, `-->`, `--`, ranges, measurements, maps or octal literals are `reject` with the code from `contracts/diagnostics.md`; a test fails if a corpus case has no entry
- [X] T042 [US2] Write `tests/corpus.rs` (SC-001, ADR-007 §2.6): for each corpus case, load its input and check the result against its entry in `corpus_expectations.toml` (graph for `accept`, code and position for `reject`), fail on a corpus case with no entry, and check that the crates.io `tree-sitter-gram` trees equal the submodule's for every corpus input

**Checkpoint**: Stories 1 and 2 pass; the loader is strict and located.

---

## Phase 5: User Story 3 - Reload an edited file into the same database (Priority: P2)

**Goal**: identified elements are matched to stored ones, not created again.

**Independent Test**: load a fixture, load an edited version, and compare element counts and property values.

### Tests for User Story 3 (write first; they must fail)

- [X] T043 [P] [US3] Create fixtures `tests/fixtures/reload/v1.gram`, `v2.gram` (changed property, added label, new element), `v-type-changed.gram`, `v-kind-swapped.gram`
- [X] T044 [P] [US3] Write `tests/contract/reload.rs` (ADR-001 §2.7): loading the same file twice leaves the graph identical (SC-003); an edited file updates matched elements, unions labels, takes the file's property values, leaves unstated properties and unmentioned elements alone, and creates new ones; a reference binds to a stored element; elements without `@id` are never matched; a relationship whose type or endpoints differ fails with `match.type-differs` and nothing is written; an element of the other kind fails with `match.kind-differs`; several stored elements with one `@id` fail with `match.db-duplicate` listing them
- [X] T045 [P] [US3] Write `tests/contract/identity_property.rs` (ADR-001 §2.10): with `node_id_property`, elements match on that property's text (integer `42` equals string `"42"`), no `@id` is written for that kind, a definition without the property is anonymous with `id.property-missing`, and two stored elements with equal text are reported

### Implementation for User Story 3

- [X] T046 [US3] In `src/load.rs`, scan existing nodes and relationships once inside the transaction (`id()`, `@id` or the identity property, relationship type and endpoint ids) and build the `IdentityTable` with duplicate tracking (research R4; data-model.md)
- [X] T047 [US3] Resolve the plan against the table in `src/plan.rs` or `src/load.rs`: match or create each element, bind references, and return the `match.*` errors before any write (ADR-001 §2.7)
- [X] T048 [US3] Write matched elements in `src/load.rs`: property merge (`SET n += r.props`) and label union in batches addressed by internal id; compare relationship endpoints by the elements they resolve to; keep creation order for new elements
- [X] T049 [US3] Implement the identity-property settings in `src/plan.rs` and `src/load.rs` (ADR-001 §2.10): text comparison in the library, no `@id` for that kind, the missing-property warning, duplicates by text

**Checkpoint**: Stories 1 to 3 pass; reloads match instead of duplicating.

---

## Phase 6: User Story 4 - Export a database to a gram file (Priority: P2)

**Goal**: any Relate database becomes a deterministic gram file, or a complete list of why not.

**Independent Test**: build a database with Cypher, export it twice, compare the outputs, and load the output.

### Tests for User Story 4 (write first; they must fail)

- [X] T050 [P] [US4] Write `tests/contract/export_basic.rs` (ADR-002 §2.4, §2.5): the layout (nodes, blank line, relationships, right arrows, labels and keys sorted), nodes and relationships in `id()` order, two exports of an unchanged database byte-equal (SC-004), the empty database, labels, relationship types and keys that need quoting (a space, a backtick, a leading `@`) written as ADR-002 §2.6 and ADR-008 §2.4 give and read back by the loader, and `insta` snapshots
- [X] T051 [P] [US4] Write `tests/contract/export_identity.rs` (ADR-002 §2.1 to §2.3, §2.7): `auto`, `generate` and `property` modes, `@id` used when present, generated `n0` and `r0` skipping collisions, identifier quoting, and failures listed together before output for a missing, unusable or duplicate identity value, an `@id` that is not a string, a property setting in `generate` mode, and `property` mode with no property
- [X] T052 [P] [US4] Write `tests/contract/export_values.rs` (ADR-006 §2.5, §2.6): spellings for integers, floats (`0.1`, `1.0`, `-0.0`, `1e300`, `1e-300`, `f64::MAX`, `f64::MIN_POSITIVE`), strings with every escape, lists, key quoting; and the unexportable cases listed together: `NaN`, an infinity (passed as parameters), an empty list, a NUL string, an empty key, a mixed list that Relate stores, a date, a point, a nested map, a `@x` key

### Implementation for User Story 4

- [X] T053 [US4] Implement the export side of `src/values.rs`: `relate::Value` to gram spelling or an `export.value` diagnostic naming the case; floats through Rust `Display` with `.0` appended when there is no dot (research R8)
- [X] T054 [US4] Implement graph reading in `src/export.rs` (research R7): one transaction, nodes and relationships ordered by `id()`, properties and labels read as `Value`
- [X] T055 [US4] Implement identifier selection in `src/export.rs` (ADR-002 §2.1 to §2.3): modes, `@id`, configured property checks with all offenders collected, generated counters that skip collisions; return `export.*` diagnostics before writing
- [X] T056 [US4] Implement the layout writer in `src/export.rs` (ADR-002 §2.5, ADR-008 §2.4) and the public `export` in `src/lib.rs`, including the `export.in-transaction` precondition

**Checkpoint**: Stories 1 to 4 pass; both directions work.

---

## Phase 7: User Story 5 - Round-trip a graph through gram (Priority: P3)

**Goal**: load then export then load gives the same graph.

**Independent Test**: for each valid fixture, compare the database loaded from the fixture with the one loaded from its export.

### Tests for User Story 5 (write first; they must fail)

- [X] T057 [P] [US5] Write `tests/roundtrip/fixtures.rs`: for every file in `tests/fixtures/valid/`, load, export, load into a fresh database and compare with `dump_graph`, up to the spelling normalizations of ADR-006 §4
- [X] T058 [P] [US5] Write `tests/roundtrip/props.rs`: `proptest` generators for property values (all accepted kinds, awkward strings, edge floats and integers) and identifier text (quotable and unquotable), checking that export then load reproduces them
- [X] T059 [P] [US5] Write `tests/roundtrip/identity.rs`: a database exported with `node_id_property` and reloaded with the same setting matches the same elements; reloading with different settings duplicates (ADR-002 §2.1); a load followed by an export is stable across two fresh databases (ADR-002 §2.4)

### Implementation for User Story 5

- [X] T060 [US5] Fix every round-trip failure the tests expose in `src/values.rs`, `src/names.rs`, `src/export.rs` or `src/plan.rs`, citing the ADR rule each fix follows; add no behavior the ADRs do not state

**Checkpoint**: all five stories pass.

---

## Phase 8: Polish and Cross-Cutting Concerns

- [ ] T061 [P] Write `benches/load_export.rs` and an `#[ignore]`d test `tests/perf.rs` that asserts SC-006 (run with `cargo test --test perf -- --ignored`): 10,000 elements load and export in under 10 seconds, including a case that alternates label sets and types on every element; record the numbers in `research.md` R5, and if the worst case misses the target implement the fallback R5 names and update R5
- [ ] T062 [P] Write `README.md`: purpose, the two entry points, settings, the diagnostic codes, the supported subset, and a pointer to the ADRs
- [ ] T063 [P] Update `CLAUDE.md` with the build, test and corpus commands
- [ ] T064 [P] Add `scripts/check-public-api.sh`, run in CI, that fails if `src/` imports anything from `relate` other than the public items research R1 lists (`Database`, `Connection`, `Value`, `QueryResult`, `Error`, `Result`), and that `Cargo.toml` has no dependency on the `relate` command-line crate (FR-013, FR-014)
- [ ] T065 Run every step of `quickstart.md` and fix any difference between it and the behavior; SC-007 (a new developer loads a sample in ten minutes) is checked by hand from the README and recorded in the pull request
- [ ] T066 Run `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` and fix findings
- [ ] T067 Confirm every user-story acceptance scenario in `spec.md` and every diagnostic code in `contracts/diagnostics.md` is covered by a test (a script or test listing each code against the tests that assert it), and list any gap in the pull request description

---

## Dependencies and Execution Order

- **Setup (Phase 1)** first. **Foundational (Phase 2)** needs Setup and blocks all stories.
- **US1** needs Foundational. **US2** needs US1's plan and loader. **US3** needs US1 (and reuses US2's errors). **US4** needs only Foundational for its tests, but its fixtures use `load` from US1. **US5** needs US1, US3 (identity settings) and US4.
- The corpus expectations (T041) need the corpus reader (T040), and the corpus runner (T042) needs both and the US1 loader.
- Within a story: tests first and failing, then values, plan, load or export, then the public surface.
- US1 is the MVP. US1 and US2 together are the strict loader (both P1).

## Parallel Opportunities

- Phase 1: T002, T003, T005 and T006 in parallel after T001; T007 (test entry files) before any test file is added.
- Phase 2: T008, T010, T012, T014 and T015 in parallel; their implementations follow.
- US1: fixtures T017 first, then tests T018 to T020 together. US2 fixtures T027 to T030 together, and T039 alongside.
- US3 tests T043 to T045 together. US4 tests T050 to T052 together; US4 and US3 can proceed in parallel after US1 if two people work on them.
- US5 tests T057 to T059 together. Polish T061 to T063 together.
- Same-file tasks are not marked `[P]`: `src/plan.rs` (T022, T023, T033, T034, T036, T037, T047, T049), `src/values.rs` (T021, T035, T053) and `src/load.rs` (T024, T038, T046, T048, T049) run in sequence.

## Implementation Strategy

1. **MVP**: Phases 1 to 3 (US1) give a loader that accepts valid files. Stop and validate with the quickstart.
2. **Strict loader**: add US2 (P1). This is the first useful release of the load side.
3. **Reload and export**: add US3 and US4 (P2), in either order or in parallel.
4. **Fidelity**: US5 (P3), then Polish.
5. Commit after each task group and cite the ADR (for example "Implements ADR-008 §2.3").
