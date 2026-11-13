# relate-gram

Load [gram](https://github.com/gram-data/gram-spec) files into a [Relate](https://relatedb.ai/)
database, and export a Relate database as gram.

Gram is a plain-text notation for graphs. `relate-gram` supports a strict subset: the constructs that
map directly to Cypher's property graph model (nodes, relationships, paths and the top-level record).
Anything else is rejected with an error that says what it is and where it is.

## Install

- **A release binary** (macOS Apple silicon and Intel, Linux x86-64): download the archive from the
  [Releases page](https://github.com/relatedb/relate-gram/releases), check it against `SHA256SUMS`, and put
  `relate-gram` on your `PATH`.
- **From source**: `cargo install --git https://github.com/relatedb/relate-gram` (Rust 1.90 or later).
- **As a library**: `relate-gram = { git = "https://github.com/relatedb/relate-gram", tag = "v0.1.0" }`. It
  works with Relate 1.0.x.

## Quick start

```bash
git clone --recurse-submodules https://github.com/relatedb/relate-gram
cd relate-gram && cargo build --release

target/release/relate-gram check people.gram                 # find every problem, with line and marker
target/release/relate-gram load people.gram --db ./people    # load it into a Relate database
relate query --database ./people "MATCH (n) RETURN n"        # query it with Relate
target/release/relate-gram export --db ./people              # write it back out
target/release/relate-gram fix people.gram --rel-type KNOWS  # correct what can be corrected, into people.fixed.gram
```

The [five-minute quickstart](docs/quickstart.md) walks through this, and
[what is supported, and why](docs/supported-gram.md) lists every construct that is rejected, its error
code, and what to write instead. `relate-gram --help` lists the commands and options.

## As a library

```rust
use relate::Database;
use relate_gram::settings::{ExportSettings, LoadSettings};

let db = Database::open(":memory:")?;
let mut conn = db.connect()?;

let source = br#"
  (ada:Person {name: "Ada"})
  (grace:Person {name: "Grace"})
  (ada)-[:KNOWS {since: 1843}]->(grace)
"#;
let report = relate_gram::load(&mut conn, source, &LoadSettings::default())?;
println!("{} nodes created", report.counts.nodes_created);

let text = relate_gram::export(&mut conn, &ExportSettings::default())?;
```

Try it: `cargo run --example load_and_query -- tests/fixtures/valid/people.gram`.

## Behavior

- **Load** (`relate_gram::load`): one transaction, so a load either completes or changes nothing.
  Identifiers in the file resolve references within it and are stored on each element as the
  reserved property `@id`; loading an edited file into the same database matches elements by
  `@id` and updates them instead of creating them again. `LoadSettings` can name a property to use
  as identity instead, choose whether a tag on a tagged string is a warning or an error, cap the
  errors listed, and give the file name for the file-kind check.
- **Export** (`relate_gram::export`): nodes, a blank line, relationships, in a deterministic order, so
  two exports of one database are identical. `ExportSettings` selects how element identifiers are
  chosen (`auto`, `generate` or `property`). A value that has no gram form (a date, a point, an
  empty list, a non-finite float) is an error that lists every offender before any text is written.
- **Errors and warnings** are `Diagnostic` values with a stable `code`, a severity, the span of the
  source text (line, column and byte offset) and a message. The codes are listed in
  [`specs/001-phase1-load-export/contracts/diagnostics.md`](specs/001-phase1-load-export/contracts/diagnostics.md).

## What is accepted

Nodes with labels and properties; relationships with exactly one type and a direction (`-[:T]->` or
`<-[:T]-`); paths; the top-level record (checked, not stored). Property values are strings,
integers, decimals, booleans and arrays of one of these. Comments are ignored. Annotations, subject
patterns, ranges, measurements, nested maps, mixed arrays, octal literals, untyped, undirected and
bidirectional relationships are errors.

## Status and limits

Version 0.1 is the first release: phase 1 of the plan, one graph space with no named graphs.
[`CHANGELOG.md`](CHANGELOG.md) lists what it contains. Known limits:

- Concurrent writers, WebAssembly and Windows are not tested; loading is tested up to 50,000 elements.
- An element written without an identifier is created again on every load. Give elements identifiers to
  have a reload update them.
- The strict subset rejects ranges, measurements, nested maps, annotations, subject patterns, and
  untyped, undirected and bidirectional relationships. `relate-gram fix` corrects the mechanical ones.
- Tags on strings (such as a date) are dropped and reported; typed tags are planned.
- Named graphs and graph metadata are planned (see `design/rfc/RFC-003-named-graphs-and-reloading.md`).

Report a problem with the [bug report template](https://github.com/relatedb/relate-gram/issues/new/choose).

## Design

The behavior is decided by the ADRs in [`design/adr/`](design/adr/) (start with the
[RFCs](design/rfc/) for the reasons). The specification, plan and tasks are in
[`specs/001-phase1-load-export/`](specs/001-phase1-load-export/). The project constitution is in
[`.specify/memory/constitution.md`](.specify/memory/constitution.md).

## Building and testing

```bash
git submodule update --init external/tree-sitter-gram   # the corpus
cargo test                                              # contract, position, round-trip, corpus tests
cargo test --release --test perf -- --ignored           # the 10,000-element performance check
cargo bench --bench load_export
```

Rust 1.96.0 is pinned in `rust-toolchain.toml`. `relate` is the published 1.0.x crate (`~1.0`).
