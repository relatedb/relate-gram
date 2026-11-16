# Research: The Conformance Collection and its Rust Runner

## R1. Reading case files

- **Decision**: `relateby_gram::parse_gram_with_header` on `case.gram` and `expected.gram`. The header record gives `requires`, `settings` and `transaction`; each remaining pattern is an expectation node (`Error`, `Warning`, `Counts`, `Applied`, `Remaining`) read from its label and properties. A pattern with an unknown label fails the case (the vocabulary check also reports it).
- **Rationale**: ADR-015 section 2.3; SPIKE-006 showed this shape reads correctly. The header is a `HashMap`, so keys are unordered and repeated keys are silent; `gram lint --strict` (R2) catches repeats.

## R2. Linting

- **Decision**: the `gram` tool (crate `gram-data`) runs as a subprocess in `conformance_checks.rs` over `case.gram` and `expected.gram`. When the tool is absent the check prints how to install it and is skipped, unless `CONFORMANCE_REQUIRE_LINT=1`, which CI sets. CI installs the tool with `cargo install gram-data --locked`.
- **Rationale**: the constitution allows only `relateby-gram` as a test-time gram dependency (ADR-015 section 2.10); calling the linter as a program adds no dependency. A skip that CI turns into a failure keeps local runs light.
- **Alternatives**: `gram-data` as a dev-dependency (rejected: another dependency, outside the ADR).

## R3. The value table

- **Decision**: the database side reads each property with `valueType(n.k)` and `toString(n.k)`; the table maps the pair to a `gram_value::Value`. T006 first prints `valueType` and `toString` for every property type the fixtures hold, and the table is written from that output: integer, float, boolean, string, list (the element type), the five temporal types and duration, point, UUID, and float vectors by kind and elements. The expected side maps the library's values: integer, decimal, bare symbol and string to string, boolean, array, and a tagged string to `Value::tagged(tag, normal_content)` through `gram_value::tags::interpret_tag`, so `10:30Z` and `10:30:00Z` meet. A float vector is compared by kind and widened elements, which the 16-bit helper of R6 decodes.
- **Rationale**: ADR-015 section 2.3; the table is the same on both sides and does not use the exporter.
- **Alternatives**: native `relate::Value` in the Rust runner (rejected: it would not test the table other languages will use).

## R4. Graph lines and comparison

- **Decision**: a node line is `:L1:L2 {k: <canonical text>, ...}` with sorted labels and keys; a relationship line is `(<start line>) -[TYPE {props}]-> (<end line>)`; two graphs are equal when the sorted lists of node lines and of relationship lines are equal. A generated identifier (an `@id` of the form `n<digits>` or `r<digits>` on an element the original had no identifier for) is dropped before the line is built when comparing a reload of an export.
- **Rationale**: ADR-015 section 2.3; `tests/common/mod.rs::dump_graph` already does this with `Debug` formatting, which the new form replaces.

## R5. Expected-graph rules

- **Decision**: `expected.gram` is checked against ADR-015 section 2.3 before use: every pattern is a node (0 elements) or a single relationship (2 elements, ends are references to top-level definitions); an annotation, a path, a map value, a repeated definition, or an end that is not defined fails the case with a message naming the rule. A subject pattern or a pattern with more than two elements fails.
- **Rationale**: the pattern-rs projection of a path differs from the grammar's documentation (SPIKE-006, SPIKE-007), so the runner accepts only forms that read the same in both.

## R6. 16-bit element conversion

- **Decision**: `src/lib.rs` re-exports `typed::elements::{widen, narrow}` in `__test_support` (doc-hidden), so the runner decodes a 16-bit embedding without a second implementation.
- **Rationale**: `typed::elements` is crate-private; `__test_support` already exists for tests (`parse`, `Syntax`). `scripts/check-public-api.sh` checks `relate::` items only.

## R7. The two passes

- **Decision**: `tests/conformance.rs` runs with `RELATE_PREVIEW` removed and `tests/conformance16.rs` sets `RELATE_PREVIEW=float16-coordinate-types` before the first connection, in a `Once`; both call the shared module. Each writes `target/conformance/results-<pass>.gram`.
- **Rationale**: the flag is process-wide; the existing `embedding16` and `embedding16_off` targets are the precedent.

## R8. Capability probes

- **Decision**: each probe is a statement in `conformance/capabilities.gram` run once per pass: `explicit-transactions` (`begin` then `rollback`), `temporal-values` (`date('2024-01-01')`), `points` (`point({x: 1.0, y: 2.0, srid: 7203})`), `uuids` (a UUID constructor), `vectors` (`vector([1.0], 1, FLOAT32)`), `vector-16-bit` (the same with `FLOAT16`). `numeric-kinds` is true for this runner because `relateby-gram` keeps `1` and `1.0` apart (SPIKE-006). Operations are declared: the Rust runner offers all four.
- **Rationale**: ADR-015 section 2.6.

## R9. Conversion of the fixtures

- **Decision**: a throwaway script run once converts the 71 `.gram` and `.expect` pairs to `reject` cases and writes `case.gram` with the `Error` nodes from the `.expect` file. For the load cases it loads each valid fixture, dumps the graph through Cypher in the form of R4, and writes `expected.gram`; every generated `expected.gram` is then reviewed against the assertions of the tests it replaces (named in `tasks.md`) before those tests are removed, so the collection does not record whatever the library happens to do as correct.
- **Rationale**: ADR-015 section 3.5 rules out expected graphs from the exporter under test; a Cypher dump is not the exporter, and the review step is the guard against a golden-master.

## R10. Which tests move and which stay

- **Decision**: moved: the position test and the two fixture-wide checks in `tests/contract/check.rs`; the per-fixture load assertions (`load_basic`, `load_values`, `load_typed`, `load_embedding`, `reload` where they are fixture-in, graph-out); the golden-string exports (`export_basic`, `export_values`, `export_typed`, `export_embedding`); the example-in, example-out fixer tests. Stay (ADR-015 section 2.8): creation order, message contents, related positions, fixer idempotence, `FixFailure`, round and annotation counters, states Cypher cannot create, the cap on errors beyond its `total`, command-line tests, the renderer, property tests, the 16-bit targets, performance, and `tests/corpus.rs`. Each moved test is named in `tasks.md` when it is removed.
- **Rationale**: SC-007.

## R11. Coverage of diagnostic codes

- **Decision**: `conformance_checks.rs` reads the codes of `relate_gram::diagnostics::code::ALL`, requires each to be the code of an `Error` or `Warning` node in some case, and takes the exceptions from a list in `conformance/README.md` with a reason (the list starts with `construct.unsupported`, which no input reaches, as `tests/coverage.rs` says today). `tests/coverage.rs` is then removed.
- **Rationale**: ADR-015 section 2.8.

## R12. Baselines

- **Decision**: T001 records the wall time of `cargo test` (debug), of the position test and of the contract tests, and the number of Rust tests, before the conversion.
- **Rationale**: SC-006 and SC-007 are relative.

## R3 results (T007, 2026-11-16): `valueType()` and `toString()` of each property type

Printed by a throwaway test against `relate` 1.0.1 (16-bit kinds with the preview flag set). `valueType` always ends in ` NOT NULL`; the table below omits it.

| Property | `valueType` | `toString` |
|---|---|---|
| integer `5`, `-0` | `INTEGER` | `5`, `0` |
| float `1.5`, `1.0`, `-0.0`, `1.0e21` | `FLOAT` | `1.5`, `1.0`, `-0.0`, `1000000000000000000000.0` |
| boolean | `BOOLEAN` | `true` |
| string | `STRING` | the string itself (not quoted) |
| list | `LIST<T NOT NULL>` (`LIST<NOTHING>` for `[]`); `[1, 2.5]` is stored as `LIST<FLOAT>` | `[a, b]`, elements unquoted, so a list is read with `[x IN n.k \| toString(x)]` |
| date | `DATE` | `2024-01-15` |
| time | `ZONED TIME` | `10:30:00+01:00`, `10:30:00Z` |
| local time | `LOCAL TIME` | `10:30:00.5` |
| datetime | `ZONED DATETIME` | `2024-01-15T10:30:00+01:00[Europe/Paris]` |
| local datetime | `LOCAL DATETIME` | `2024-01-15T10:30:00` |
| duration | `DURATION` | `P1Y2M3DT4H5M6.789S` |
| point | `POINT` | `point({x: 1.5, y: 2.0, crs: 'wgs-84'})`: a crs name, not an SRID, so a point is read with `.srid`, `.x`, `.y`, `.z` |
| UUID (`uuid('...')`) | `UUID` | lower-case `8-4-4-4-12` |
| vector | `VECTOR<K NOT NULL>(n)` with `K` one of `FLOAT32`, `FLOAT` (for FLOAT64), `INTEGER8` and the other integer kinds, `FLOAT16`, `BFLOAT16` | `vector([1.0, 2.5], 2, FLOAT32)`; `FLOAT` is written for 64-bit |

Notes: `randomUUID()` returns a STRING, so the capability probe for UUIDs uses `uuid('...')` (corrected in `conformance/capabilities.gram`). The strings in the printout of T007 showed Rust `Debug` escapes; the stored text is not escaped.
