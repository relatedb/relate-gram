# The conformance collection

Cases that state what relate-gram accepts, rejects, loads, exports and fixes, written so that an
implementation in another language can run the same files. The decision is ADR-015 (design/adr);
this file is the guide to the files. Version: `VERSION`. The grammar the cases were written against:
`GRAMMAR_VERSION`.

## Case form

A case is a directory `conformance/<kind>/<id>/`. The kinds:

| Kind | Files | Runs |
|---|---|---|
| `reject` | `case.gram`, `input.txt` or `input.bin`, optional `start.cypher` | `check` and `load` |
| `load` | `case.gram`, `input.txt`, `expected.gram`, optional `start.cypher`, optional `exported.txt` | `load`, and `export` with `exported.txt` |
| `export` | `case.gram`, `start.cypher`, `exported.txt` or `Error` nodes | `export` |
| `fix` | `case.gram`, `input.txt`, `fixed.txt` | `fix` |

`case.gram` has a header record with `requires` (capabilities), optional `settings` (lowerCamelCase
keys) and optional `transaction: true`, then expectation nodes: `Error`, `Warning`, `Counts`,
`Applied`, `Remaining`. `input.txt`, `input.bin`, `exported.txt` and `fixed.txt` are byte strings, never
parsed; `.gitattributes` keeps a checkout from changing their line endings. `expected.gram` is the graph
after a load: nodes defined once at the top level, single-hop relationships whose ends are named by an
identifier and defined earlier. A `reject` case without `start.cypher` uses `conformance/start.cypher`.

Details beyond ADR-015, found while converting the Rust tests:

- A `reject` case whose errors include a `match.*` code, or that requires `!vector-16-bit`, is not
  checked without a database, because `check` cannot report those.
- A `fix` case may carry the loader's `tags` setting beside the fixer's options.
- Cypher in `start.cypher` has no `\r`, `\b` or `\f` string escapes (Relate keeps the backslash); use
  `\u000D`, `\u0008` and `\u000c`.
- Annotation cases carry the marker text, message included, in `fixed.txt`.

## Comparing graphs and values

Both sides of a graph comparison reduce each property value to a gram-value `Value` and compare canonical
lines as multisets, ignoring generated identifiers. The database side reads a property with
`valueType()` and `toString()` (neither side uses the exporter):

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

On the expected side an integer, decimal, boolean, string or bare symbol, an array, and a tagged string
(the tag's normal content, so `10:30Z` and `10:30:00Z` meet) map to the same values.

## Types an expected graph can hold, and the case that holds each

| Type | Case |
|---|---|
| integer, float, boolean, string, list, empty list | `load/values`, `load/empty-array` |
| date, time, local time, datetime, local datetime, duration | `load/typed-*` |
| point (2D, 3D, lists of points) | `load/typed-point`, `load/typed-lists` |
| UUID | `load/typed-uuid` |
| vectors (float32, float64; 16-bit with the flag) | `load/embedding-*`, `load/embedding16-*` |

Integer vectors cannot be written in gram; `export/value-integer-vector` covers them.

## Vocabulary and capabilities

`vocabulary.gram` lists the labels and capability names a case may use; the collection checks fail on
any other. `capabilities.gram` gives the probe for each capability: a Cypher statement that must
succeed, an explicit transaction, a property of the runner's gram reader, or an operation the
implementation declares. A requirement may be negated with `!`. The 16-bit probe depends on Relate's
process-wide `RELATE_PREVIEW=float16-coordinate-types`, so a run has two passes. A declared file,
`declared-capabilities.gram`, may only remove a capability: `(:Capability {name, supported: false,
reason})`. `known-failures.gram` lists `(:KnownFailure {case, reason, issue})`; such a case must still
fail, and an unexpected pass fails the run.

## Running

```bash
cargo test --test conformance      # flag unset; writes target/conformance/flag-unset.gram
cargo test --test conformance16    # flag set; writes target/conformance/flag-set.gram
CONFORMANCE_REQUIRE_LINT=1 cargo test --test conformance_checks
```

Each results file has one `(:Result {case, status, version})` per case; `status` is `passed`,
`failed`, `skipped` or `not-applicable`. Together the two files cover every case. The checks need the
`gram` tool for the lint step (`cargo install gram-data --locked --version 0.7.0`); without it the lint
is skipped unless `CONFORMANCE_REQUIRE_LINT=1`.

## Tests that stay in Rust

- Message contents, the related positions of a diagnostic, the order in which elements are created, and
  the library's internals and public types.
- Fixer rounds and annotation counters, descriptions, refusal of a fix that makes things worse, and
  the check that a fixed file loads (`tests/fix.rs`). Idempotence of every `fix` case runs in
  `tests/conformance.rs`.
- Export states Cypher cannot create (NaN, infinities, NUL, empty key), idempotence, no modification of
  the database, the transaction state after a refusal, and the dtype and integer-vector messages
  (`tests/contract/export_*.rs`, `export_embedding.rs`).
- The float bit-for-bit round trip, the limit messages, the cap on reported errors beyond its `total`,
  settings variants, and duplicate stored identifiers the file does not touch (`tests/contract/`).
- The 16-bit tests that assert message text (`tests/embedding16*.rs`), the command-line tests, the
  renderer, property tests, performance, and `tests/corpus.rs`.

## Codes without a case

Each diagnostic code of the library has a case, except these, each with its reason:

- `construct.unsupported`: a guard against a grammar upgrade; no input of the pinned grammar reaches it, and a tree-sitter node cannot be built by hand.
- `load.database-error`: a failure of the database itself while writing; no input makes Relate fail that way.
