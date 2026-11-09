# Changelog

All notable changes to relate-gram are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project follows
[Semantic Versioning](https://semver.org/spec/v2.0.0.html); until 1.0, a minor version may change the
command line or the library interface.

## [0.1.0] - 2026-11-11

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
- The build depends on a pinned commit of `relatedb/relate` until Relate's public release.
