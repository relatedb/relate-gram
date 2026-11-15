# Research: Known Tags from gram-value

## R1. Adding the dependency

- **Decision**: `gram-value = "0.1"` in `[dependencies]`; no features. The crate has no runtime dependencies and `rust-version` 1.75, below this crate's 1.90.
- **Rationale**: ADR-012 section 2.1. A cargo tree check (task T002) confirms exactly one new package.
- **Alternatives**: a path or git dependency (rejected: published crate exists, 0.1.1).

## R2. Mapping a diagnostic to `value.tag-content`

- **Decision**: `interpret_tag(tag, content)` returns `Result<Option<Interpretation>, Diagnostic>`. `Some` for the ten known tags, `None` otherwise. relate-gram first filters on `Tag::from_name` (eight tags), so `week` and `weekDate` never reach it and stay unrecognized. A `Diagnostic` carries a message prefixed `"{tag}: "`; the adapter strips that prefix and drops the code (`invalid-form` and `out-of-range` are not distinguished).
- **Rationale**: ADR-012 sections 2.2 and 2.5; the existing message wrapper in `values.rs` already names the tag.
- **Open check**: the exact field names of `Diagnostic` are read from the crate's source in task T004.

## R3. What the loader passes to Relate

- **Decision**: text tags use `Interpretation::normal_content` (seconds present, `Z` for `+00:00`, trimmed fraction, lower-case UUID); a zone name stays as written inside it. For `point`, `Typed::Point(Point { srid, x, y, z })` fills the existing `PointContent` (`srid` as `i64`), after the profile checked the SRID.
- **Rationale**: ADR-012 section 2.2. `typed_failure` matches database messages against the text passed, so `typed_text` returns the normal content.
- **Alternatives**: pass the original text (rejected: `time('10:30Z')` is accepted by Relate per SPIKE-005 but the normal form is the one gram-value tests).

## R4. Comparing old and new validators

- **Decision**: before deletion, copy the old `src/typed/{temporal,point,uuid}.rs` into a test-only module (`tests/typed_compare/old/`), run both over every `tests/fixtures/{valid,invalid}/typed-*` and `tag-content-*` input and over the cases extracted from the tables in `specs/002-typed-tags/` (data-model and research), and write the accept/reject pairs to the spec's Expected Differences table. Delete the old copy and the comparison test afterwards; the inputs stay as fixtures.
- **Rationale**: ADR-012 section 2.7; a copy in `tests/` avoids keeping dead code in `src/`.
- **Alternatives**: compare by git checkout of two builds (rejected: slower and harder to commit as evidence).

## R5. Exporting a point

- **Decision**: Rust's `Display` for `f64` is the shortest decimal that reads back equal and never uses an exponent; `-0.0` prints `-0`, which the writer turns into `0`. The writer emits `point`SRID={srid};{x} {y}[ {z}]``. The SRID and finiteness checks of `point_text` stay.
- **Rationale**: ADR-012 section 2.6; matches gram-value's number-in-point writing.
- **Alternatives**: reuse `float_text` (rejected: it keeps `.0`, which gram-value's canonical point text does not write).

## R6. Performance baseline

- **Decision**: task T001 records the output of the typed case of `tests/perf.rs` (and `cargo bench --bench load_export`) on this machine before any change, in a note in the task list; SC-004 compares against it.
- **Rationale**: SC-004 is relative; the existing test only asserts "under 10 times plain strings".
