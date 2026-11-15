# Changelog

All notable changes to relate-gram are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project follows
[Semantic Versioning](https://semver.org/spec/v2.0.0.html); until 1.0, a minor version may change the
command line or the library interface.

## [Unreleased]

### Added

- Typed tags (ADR-011): a tagged string with the tag `date`, `time`, `localtime`, `datetime`,
  `localdatetime`, `duration`, `point` or `uuid` loads as a value of the matching Relate type, an array of one
  such tag loads as a list of that type, and typed values export as tagged strings, so a database holding them
  can be exported and read back with equal values. The content forms are in `docs/supported-gram.md`. Their validation comes from the
  `gram-value` crate (ADR-012), narrowed by a Relate profile: a time or datetime needs an offset, and a point needs
  an SRID of 4326, 7203, 4979 or 9157. Seconds are optional. A point is written ``point`SRID=4326;12.5 55.7` ``
  (three numbers for 4979 and 9157), the form other gram tools use.
- The diagnostic code `value.tag-content`: the content of a recognized tag does not follow its form. `check`
  finds it without a database, except whether a zone name exists and matches its offset, which only a load can
  tell.
- Library: the new `typed` module holds the tags and the profile; the crate now depends on `gram-value` 0.1
  (ADR-012). Property values are held as `gram_value::Value` (ADR-013, below).

### Changed

- Property values are held and written through `gram-value` (ADR-013). A string property that is a symbol is
  now exported bare (`status: active`, not `status: "active"`); any other string stays double-quoted, and a
  leading `/` in a string value is no longer escaped. Integers, floats, booleans, lists, points and the other typed
  values are written as before. Library items removed or changed against 0.1.0: `values::PropValue` is replaced
  by `values::Lowered` (a `gram_value::Value` with positions), `plan::PlannedNode.props`,
  `plan::PlannedRel.props` and `plan::value_text` use it, and `names::string_literal` is removed.
- A file that wrote one of those eight tags on a string used to load the content as a string with a warning. It
  now loads a typed value, or fails with `value.tag-content` if the content does not follow the form. To keep a
  plain string, remove the tag or use another one. The tree-sitter-gram corpus itself writes ``time`17:35` `` and
  ``point`30 10` ``; neither is accepted (a time needs an offset, a point needs an SRID), but ``time`17:35Z` `` loads.
  Extended well-known text (``point`SRID=4326;POINT(1 2)` ``) is not accepted either; write ``point`SRID=4326;1 2` ``.
- Vectors still cannot be exported; the error now names the vector.

### Known limits

- A UUID loaded into a persisted database reads back as a string after the database is reopened, because this
  crate builds Relate without its default `parquet` snapshot codec (SPIKE-005 notes). Dates, times, datetimes,
  durations and points survive a reopen.

- Built against tree-sitter-gram 0.7.0. An empty array (`{tags: []}`) now parses: it loads as an empty list
  and an empty list exports as `[]` (it used to be a syntax error and an export error). A measurement with a
  decimal magnitude (`3.5kg`) parses and is rejected as a measurement, like `168cm`.

## [0.1.0] - 2026-11-13

First release: gram support for Relate, phase 1 (one graph space; see `design/rfc/RFC-003-*.md` for the
later phases).

### Added

- **Load** gram text into a Relate database in one transaction (`relate_gram::load`): nodes, relationships,
  paths and the top-level record, with a strict subset that rejects anything that does not map directly to
  Cypher. Identifiers are stored as `@id`, and loading an edited file into the same database matches
  elements instead of duplicating them. An identity property can be used instead of `@id`.
- **Export** a Relate database as gram (`relate_gram::export`), deterministically, with `auto`, `generate`
  and `property` identifier modes. A value with no gram form is an error that lists every offender.
- **Errors with positions.** Every error and warning has a stable code, a line, column and byte offset, and a
  message, and is rendered with the source line and a marker (`relate_gram::render`).
- **`relate-gram` command**: `check`, `load`, `export`, `normalize` and `fix`.
- **`relate-gram fix`** corrects the errors that have a mechanical correction (missing relationship types,
  undirected and bidirectional arrows, fat and wavy arrows, `::`, octal literals, mixed number arrays, tags),
  writes a new file by default, and marks the errors it cannot fix with `// relate-gram:` comments.
- Documentation: a five-minute quickstart (`docs/quickstart.md`) and a list of what is supported and why
  (`docs/supported-gram.md`).

### Known limits

- Concurrent writers, WebAssembly, and Windows are not tested. Loading is tested up to 50,000 elements.
- Relationships and nodes written without an identifier are created again on every load.
- Ranges, measurements, nested maps, annotations, subject patterns and tags with a meaning (dates, points)
  are not supported; named graphs are planned.
- Built and tested against Relate 1.0.1 (requirement `~1.0`).
