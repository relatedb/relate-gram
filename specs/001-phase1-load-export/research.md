# Research: Phase 1 Gram Load and Export

Decisions the plan needs that the ADRs leave to implementation. None of them reopens an ADR. Items marked "measure" are checked by a task in this feature.

## R1. How the crate depends on Relate

- **Decision**: a git dependency on `https://github.com/neo4j-labs/cypherlite` pinned by `rev` to the commit the spikes ran against (`84859b14`, `relate` 0.16.1), using the `relate` package from that workspace. Local development against a checkout uses a `[patch]` override that is not committed.
- **Rationale**: the `relate` on crates.io is version 0.1.0, not the 0.16.1 that SPIKE-001 and SPIKE-002 measured. Pinning a commit makes the measured behavior the tested behavior.
- **Alternatives**: a path dependency (breaks clean checkouts and CI); waiting for a crates.io release (blocks the project).
- **Features**: `relate` enables a parquet snapshot codec by default, which this crate does not use. Try `default-features = false` first and keep it if the crate builds and the spike probes still pass; record the outcome here. **Outcome (T001)**: `default-features = false` builds; `relate` is used without the parquet codec.
- **Consequence**: building needs network access to the git remote, and bumping the pin is a deliberate change that reruns the spike-derived tests.

## R2. Toolchain

- **Decision**: pin Rust 1.96.0 in `rust-toolchain.toml`, with `rustfmt` and `clippy` components, and set the crate's `rust-version` to 1.90.
- **Rationale**: Relate's own `rust-version` is 1.88, but its transitive dependencies `tree-sitter-language` 0.1.9 and `ordered-float` 5.5 require 1.90, and 1.88.0 fails to resolve them (found while implementing T001). 1.96.0 is the stable release the project was built with. The first plan pinned 1.88.0.

## R3. Parsing layer

- **Decision**: `tree-sitter` 0.25 with the `tree-sitter-gram` 0.6.1 crate from crates.io as the build dependency. The submodule at `external/tree-sitter-gram` (tag v0.6.1) supplies only the corpus files, and a test checks that the crate's trees equal the submodule's for every corpus input (ADR-007 §2.6).
- **Syntax layer**: `syntax.rs` parses a byte slice, rejects non-UTF-8 input with the offset of the first bad byte, skips a UTF-8 byte order mark, and, if the root `has_error`, walks the tree for ERROR and MISSING nodes and returns them as diagnostics. Otherwise it exposes typed views (node, relationship, record, property, value) over tree-sitter nodes. Positions are the node's start row plus one, start column (a byte count) plus one, and byte offset; the range end is kept for later fixes.
- **Evidence**: SPIKE-003.

## R4. Load pipeline and the transaction

- **Decision**: load runs in four phases.
  1. *Parse and plan* with no database access: build the `LoadPlan` from the tree, apply ADR-006 value rules, ADR-008 relationship rules and ADR-001 file-scope rules (kinds, single definition, endpoints, anonymous elements). All diagnostics from this phase are returned together, and a failing plan stops the load.
  2. *Begin* a transaction on the caller's connection (the caller must not already be in one: a precondition error).
  3. *Scan* existing nodes and relationships once (`id()`, `@id` or the identity property, relationship type and endpoint ids), build the identifier table in the library, resolve the plan against it, and report matching errors (ADR-001 §2.7). If there are errors, roll back.
  4. *Write* in batches (R5), then `commit`. A failed statement is followed by `rollback` only (SPIKE-001 §4.4).
- **Rationale**: ADR-001 §2.7; SPIKE-001 §4.6 (a label-free lookup is a full scan, a single scan is 66 ms per 50,000 nodes).

## R5. Batched writes

- **Decision**: elements are created in order of first appearance (ADR-001 §2.9) in batches of consecutive elements that share a shape, so ids follow appearance order exactly. For nodes the shape is the label set; for relationships it is the type. Each batch is one statement with an `UNWIND $rows` and a parameter list, capped at 5,000 rows:
  - create nodes: `UNWIND $rows AS r CREATE (n:`A`:`B`) SET n = r.props`, labels written into the text with backtick quoting (R6);
  - create relationships: `UNWIND $rows AS r MATCH (a) WHERE id(a) = r.s MATCH (b) WHERE id(b) = r.e CREATE (a)-[x:`T`]->(b) SET x = r.props`;
  - update a matched element: `UNWIND $rows AS r MATCH (n) WHERE id(n) = r.id SET n += r.props`, with label additions grouped by the labels to add.
  Property maps are parameters (`SET n = $props` works for any key, including `@id`: SPIKE-001 §4.1). Newly created nodes need their internal ids for the relationship batches, so node batches return `id(n)` in row order and the loader maps plan elements to ids.
- **Measured (T061, release build, in-memory database)**: 10,000 elements (5,000 nodes and 5,000 relationships) load in 0.15 s and export in 0.04 s; the worst case, with label sets and types alternating on every element, loads in 0.5 to 0.7 s. 50,000 elements load in 0.7 s (2.6 s alternating) and export in about 0.2 s. The fallback below is not needed.
- **Measure (planned)**: a file that alternates label sets or types on every element forces one statement per element. The benchmark (SC-006) includes that worst case. If it misses the target, the fallback is to create all nodes unlabelled in one statement and add labels per group, which keeps creation order; the plan then records that as a change to this decision.
- **Alternatives**: grouping all elements of a shape together regardless of order (cheaper, but ids stop following appearance order, contradicting ADR-001 §2.9); per-element statements (simple, slow).

## R6. Names in query text

- **Decision**: labels and relationship types cannot be parameters, so `names.rs` writes them into the statement as backtick-quoted names with a backtick doubled, after rejecting any name that contains a NUL. Property keys and values always travel as parameters.
- **Rationale**: a file controls these names, so they must never be spliced raw.

## R7. Export reads

- **Decision**: one transaction (`begin`, two reads, `rollback`, since nothing is written) reading `MATCH (n) RETURN id(n), labels(n), properties(n) ORDER BY id(n)` and `MATCH (a)-[r]->(b) RETURN id(r), type(r), properties(r), id(a), id(b) ORDER BY id(r)`. Rows hold `relate::Value`, which export checks against ADR-006 §2.6 before writing any output.
- **Rationale**: ADR-002 §2.4. Relate returns a result as a whole, so a very large database is held in memory during export (ADR-002 consequences).

## R8. Float spelling on export

- **Decision**: use Rust's `Display` for `f64`, which prints the shortest digits that read back as the same value and never uses an exponent, then append `.0` if there is no dot. `-0.0` prints as `-0` and gets `.0`. Tests cover `0.1`, `1e300`, `1e-300`, `-0.0` and values at the edges.
- **Rationale**: ADR-006 §2.5 wants the shortest round-tripping decimal without an exponent.

## R9. Diagnostics

- **Decision**: one `Diagnostic` type for every error and warning, with a stable code, a severity, a source range (start and end), a message, and optional related ranges (for example the first definition of a duplicate). The codes are listed in `contracts/diagnostics.md`.
- **Rationale**: Constitution VI needs positions asserted in tests, and the fixer note asks for codes and ranges so fixes can be added later.

## R10. Tests

- **Decision**: contract tests drive the public API against fresh in-memory databases; position tests carry a fixture per diagnostic code with the expected line, column and offset; the corpus runner reads the submodule's `test/corpus/*.txt` files in `tree-sitter test` format (a small reader in the test crate) and, for each input the subset accepts, compares the load plan to the tree; round-trip tests use fixtures plus `proptest` generators for property values and identifiers; exact export text is asserted inline.
- **Alternatives**: depending on pattern-rs's corpus reader (ADR-007 removes that dependency).

## R11. Not in this plan

- The command-line integration, language bindings, WebAssembly and the fixer (design note 2026-11-09); settings are library inputs until the CLI is decided.
- Persistent stores, reopening a store and concurrent writers (SPIKE-001 left them open). The first one to matter is reopening: ADR-002 relies on id order being stable.
