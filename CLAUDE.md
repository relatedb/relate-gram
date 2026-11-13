# CLAUDE.md

## Purpose

`relate-gram` is a standalone, out-of-tree Rust crate that adds a strict subset of the
[gram](https://github.com/gram-data/gram-spec) notation to [Relate](https://relatedb.ai/) (the embedded Cypher graph
database in `relatedb/relate`, formerly `neo4j-labs/cypherlite`): load a gram file into an in-memory Relate database, and export one back out.
It links the public `relate` crate as a library and integrates minimally with the `relate` CLI (`relate run`,
ADR-0429/0431 upstream). It may be brought in-house later.

## Scope (decided in the kickoff conversation)

- Gram graph notation only: nodes, relationships, paths, and the top-level record. No annotations, no pattern notation.
- **Strictness rule:** reject anything that does not map directly to Cypher, with a located error. Property values are
  string, integer, float, boolean and homogeneous arrays of scalars; no nested maps, ranges or
  measurements. A tagged string with one of eight tags (`date`, `time`, `localtime`, `datetime`, `localdatetime`,
  `duration`, `point`, `uuid`) loads as a typed value (ADR-011); any other tag is dropped and reported (ADR-006).
  Vectors cannot be written in gram. Nodes may have several labels; a relationship has exactly one type.
- **Phases** (RFC-003 §5.9): the first phase is one unconstrained graph space with no named graphs, no ontology and no metadata nodes; named graphs and their metadata come second, behind `RELATE_PREVIEW=ontology`; meta nodes are third.
- The top-level record: in the first phase it is checked (`kind`) and its properties are reported as not stored (ADR-003). Later, graph metadata (name, digest, record properties) lives in the ontology document under `extensions.custom.relate-gram`, and the third phase materializes it as `Graph` meta nodes in the reserved graph `@@` (ADR-005, proposed).
- **Assumption:** only gram kind `property-graph` is supported (assumed when absent; other kinds are errors that say so). Files follow `graph-name.kind.gram`; the kind is checked in the first phase, and from the second phase the graph name comes from the record's `name`, else the file name (ADR-003, ADR-005).
- **Identity:** gram IDs resolve references while loading and are kept as the element's `@id`, or identity comes from a property the caller names (ADR-001; ADR-004, accepted, amends Constitution III). No identifier is generated for anonymous elements in the first phase. The first appearance of an ID fixes it
  as a node or a relationship; using it later as the other kind is an error. An element is defined at one site (labels and properties) and may be referenced anywhere else by its identifier alone; a second definition is an error.
- Second phase: one Relate database holds several graphs, distinguished by named graphs, with membership recorded per element (RFC-003). Elements keep their `@id`, which
  round-trips to gram by becoming the element identifier. Also: managed multi-graph load, and merging
  graphs within one named graph.
- Parser: `tree-sitter` with `tree-sitter-gram` 0.7.0, read directly, not through `gram-codec` (ADR-007, which also
  ignores all comments in data files); tree-sitter-gram's corpus is the conformance reference.

- **Export IDs:** modes `auto` (default), `generate`, `property`. `auto` uses `@id` if present, else a configured
  id-property if present and unique, else a sequential ID (`n0…` for nodes, `r0…` for relationships, deterministic
  order). If the id-property is missing on some elements or not unique, export errors and lists the offenders (no silent
  fallback). The property stays in the exported record. Non-symbol identifiers are quoted automatically.

Still open: design constraints for the TypeScript and Python bindings.

## Reference repos (local, read-mostly)

- `/Users/abk/Developer/neo4j-labs/cypherlite`: a local checkout of Relate (the upstream is now `relatedb/relate`). Its `AGENTS.md` and `extensions/` show the extension contract.
- `/Users/abk/Developer/relateby/pattern-rs`: `gram-codec` and related crates (not a dependency; see ADR-007).
- `/Users/abk/Developer/gram-data/tree-sitter-gram`: the grammar and its test corpus.

## Design process: RFC → ADR → implementation (with spikes)

Adapted from `/Users/abk/Developer/akollegger/graph-web/design/`. Skills live in `.agents/skills/` and are symlinked
from `.claude/skills/`.

- **RFC** (`design/rfc/RFC-NNN-*.md`, `/rfc-create`, `/rfc-review`): the WHAT and WHY, with candidate directions. No implementation detail.
- **Spike** (`design/spikes/SPIKE-NNN-*/SPIKE.md`, `/spike-create`): a time-boxed investigation of one empirical question.
- **ADR** (`design/adr/ADR-NNN-*.md`, `/adr-create`, `/adr-review`): one technical decision, always with a parent RFC.
- **Notes** (`design/notes/YYYY-MM-DD-*.md`): freeform, non-binding.
- Don't hand-edit the index tables in the `design/*/README.md` files; the skills maintain them.
- Implementation runs through Spec Kit (`/speckit-specify`, `-plan`, `-tasks`, `-implement`), installed in `.specify/`. Specs
  implement accepted ADRs and name them rather than restating them; the ADR wins on conflict. Commits and PRs cite the ADR.
  The constitution is `.specify/memory/constitution.md`; every plan checks itself against it.
- Write design documents for a newcomer: name project artifacts on first use.

## Command

`relate-gram check|load|export|normalize|fix` (ADR-009 and ADR-010, `src/main.rs`; tested by `tests/cli.rs`). The fixer is `src/fix.rs`, tested by `tests/fix.rs`: when a rejection rule or error code is added or changed, decide whether it has a fix (ADR-010 section 4). User docs are in
`docs/`: `quickstart.md` and `supported-gram.md`. When a rejection rule or a diagnostic code changes, update the
table in `docs/supported-gram.md`.

## Build and test

- `git submodule update --init external/tree-sitter-gram` fetches the grammar corpus (needed by `tests/corpus.rs`).
- `cargo test` runs the unit, contract, position, round-trip and corpus tests. A test target in a folder needs
  `main.rs` (`tests/contract/`, `tests/positions/`, `tests/roundtrip/`); add a `mod` line when adding a test file.
- `cargo test --release --test perf -- --ignored --nocapture` checks SC-006; `cargo bench --bench load_export` prints timings.
- `scripts/check-public-api.sh` checks that only Relate's public items are used (Constitution V).
- `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` must pass before merge.
- Work from `specs/001-phase1-load-export/tasks.md`; tick tasks off as they are done, and cite the ADR in commit messages.
