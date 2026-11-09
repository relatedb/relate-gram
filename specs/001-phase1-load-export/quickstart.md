# Quickstart: Validating Phase 1

Runnable checks that show the feature works end to end. Prerequisites: Rust 1.88.0 (pinned by `rust-toolchain.toml`), network access to fetch the pinned `relate` commit, and the `tree-sitter-gram` submodule.

```bash
git submodule update --init external/tree-sitter-gram
cargo test
```

## 1. Load and query (Story 1)

```bash
cargo run --example load_and_query -- tests/fixtures/valid/people.gram
```

Expected: the example opens an in-memory database, loads the file, runs a Cypher query that counts nodes and relationships, and prints the counts and the warnings. The counts match the fixture's comments. See [contracts/library-api.md](contracts/library-api.md).

## 2. Rejection with positions (Story 2)

```bash
cargo run --example load_and_query -- tests/fixtures/invalid/untyped-relationship.gram
```

Expected: the load fails with `rel.no-type`, naming the line and column of the arrow, and the example reports that the database is unchanged. Every code in [contracts/diagnostics.md](contracts/diagnostics.md) has a fixture under `tests/fixtures/invalid/`, checked by `tests/positions`.

## 3. Reload (Story 3)

```bash
cargo test --test contract reload
```

Expected: loading a fixture twice leaves the database identical; loading an edited fixture over it updates matched elements and creates new ones; a failing second load changes nothing.

## 4. Export and round trip (Stories 4 and 5)

```bash
cargo test --test contract export
cargo test --test roundtrip
```

Expected: two exports of one database are byte-equal; load, export and load again into a fresh database give the same graph for every fixture, up to the spellings ADR-006 §4 lists.

## 5. Corpus conformance (SC-001)

```bash
cargo test --test corpus
```

Expected: every corpus input the subset accepts loads into the graph the corpus tree describes, every excluded input fails at the expected node, and the crates.io `tree-sitter-gram` trees equal the submodule's.

## 6. Performance (SC-006)

```bash
cargo bench --bench load_export
```

Expected: 10,000 elements load and export in under 10 seconds, including the worst case of alternating label sets.
