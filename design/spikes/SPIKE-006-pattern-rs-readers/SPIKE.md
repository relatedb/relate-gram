---
id: SPIKE-006
title: Reading Gram with the pattern-rs Libraries
status: done
rfcs: [RFC-005]
created: 2026-11-15
---

# SPIKE-006: Reading Gram with the pattern-rs Libraries

## 1. Question

Can a conformance runner (ADR-015) read case files and expected graphs with the gram libraries that pattern-rs publishes (`relateby-gram` for Rust, `@relateby/gram` for npm, `relateby.gram` for Python, all version 0.7.2), and where do those libraries lose or change what a gram file says? The wider question is how comprehensive the ecosystem's gram libraries are for tools that must be faithful to the notation.

## 2. Method

Eighteen small inputs, then thirty with later additions, in `inputs/`: `ok-*` files that `gram lint --strict` (the reference grammar, tree-sitter-gram) accepts, and `bad-*` files that it rejects. Each language has a probe (`rust/`, `ts/`, `py/`) that reads every input with the library's header-aware parse and prints a canonical description (the record, then each pattern with identity, labels and typed property values). The outputs are `output-rust.txt`, `output-ts.txt` and `output-py.txt`. The probes use the published packages from crates.io, npm and PyPI, and Python 3.11.

## 3. Time-box

Three hours.

## 4. Findings

### 4.1 What works

- **The case-file shape of ADR-015 reads correctly in all three languages.** A leading doc comment is ignored; the top-level record comes back separately with a nested map (`settings`), an array (`requires`) and integers kept as integers; labeled nodes come back with their properties.
- **Expected-graph content reads correctly.** Integer and decimal literals are distinct (`1815` and `1.0` stay apart in Rust and Python); tagged strings come back with tag and content (`date`, `point`, `uuid`, `embedding`); bare symbols and quoted strings are distinct (`sym:active` and `str:"active"`); strings with escapes, Unicode, single quotes and a fenced string with its body are exact; `-0` reads as 0; hexadecimal reads; arrays, empty arrays and shallow maps read; identifiers (symbol, integer, backtick-quoted, anonymous), `::` in a record (`{kind:: "load"}`), quoted and backtick keys, a bare `true` key and backtick labels all read.
- **Relationships and paths** come back as a pattern for the relationship holding its two end patterns: `(a)-[:R {w: 1}]->(b)-[:S]->(c)` nests, and `(x)<-[:T]-(y)` is `T` with `[y, x]`, so direction is the order of the elements.
- **Rust and Python agree on every input** except the text of error messages, the debug format of ranges and measurements, and the order of keys inside an annotation's map value (4.2). TypeScript is the same core compiled to WebAssembly, so it agrees except where JavaScript numbers differ (4.3).
- **Malformed input is reported** for a missing value, an unterminated string, a stray token, a block comment, a top-level comma, a quoted-string identifier. Syntax errors carry a line and column, but at the start of the enclosing construct.
- **`gram lint --strict` agrees with the intent** for every `ok-` and `bad-` file (the check that classified them).

### 4.2 Gaps in the Rust core, seen in all three languages

- **Duplicate keys are merged silently.** `(a {k: 1, k: 2})` is `k = 2` and `{kind: "reject", kind: "load"}` is `kind = "load"`. The reference linter reports both as `duplicate-key` errors.
- **A fenced tag is dropped silently.** `` ```date`` ``-style fenced content with a tag after the fence comes back as a plain string (`str:"2024-01-15\n"`), losing the `date` tag with no error.
- **Arrow kind is lost.** `-->`, `==>`, `~~>`, `--` and `<-->` read as the same anonymous pattern holding two elements; undirected, bidirectional, fat and wavy arrows cannot be told from a directed thin arrow.
- **`::` before a label is a syntax error** (`(a::Wide)`), though the grammar accepts it. `::` in a record property is accepted.
- **Annotations are flattened into properties of a wrapping anonymous pattern, and a map value becomes a string** whose key order varies between runs (`"{line: 2, code: ...}"` in one run and `"{code: ..., line: 2}"` in another), because the map is a hash map.
- **Integers beyond 64 bits are a syntax error** whose position is the start of the record, not the value; a decimal with too many digits is rounded to the nearest float without notice.
- **An empty record `{}` is read as an anonymous empty pattern, not as a header.**
- **No positions are returned** for values, so a tool that must point at a value (a loader, a linter) cannot use the result. SPIKE-003 reached the same conclusion for the loader.
- **Ranges and measurements hold floats**, so `5kg` and `5.0kg` are the same value, as SPIKE-001 of `gram-value` found for the models it compared.
- **Two value models exist in the ecosystem.** pattern-rs's `Value` has `VInteger(i64)`, `VDecimal(f64)`, a separate symbol and string, and float measurements; `gram-value`'s model has eleven kinds including big integers and big decimals, no symbol kind, and one canonical text. pattern-rs does not depend on `gram-value`, so nothing converts between them or shares a canonical spelling.

### 4.3 Gaps specific to TypeScript and Python

- **TypeScript collapses `1.0` into `1`.** `score: 1.0` comes back as an integer value, because the WebAssembly boundary passes plain JSON numbers. A case that distinguishes an integer from a decimal cannot be read there.
- **TypeScript cannot read an integer above 2^53**: the whole file fails (`9223372036854775807 can't be represented as a JavaScript number`), including every other value in it.
- **TypeScript's header record is plain JSON**: `maxErrors: 100` has no integer-or-decimal kind. The Rust and Python headers keep the kind.
- **The Python distribution declares `requires-python = ">=3.8,<3.14"`**, so it does not install on Python 3.14 (pip reports no matching distribution); the spike ran on 3.11.
- **The npm package needs an initialization step** (`await init()`) and the `effect` library as a peer; this worked in Node.js 24 without further setup. A browser was not tried.

## 5. Conclusion

For ADR-015's case files and expected graphs, the pattern-rs libraries are usable today, with three precautions: run `gram lint --strict` over the collection in CI (the libraries accept duplicate keys and drop fenced tags silently, and the linter catches both); keep integers within 2^53 and avoid relying on `1.0` versus `1` in TypeScript; and use Python at or below 3.13.

They are not a faithful reader of gram. Duplicate keys, fenced tags, arrow kinds, annotations, `::` labels, big integers and positions are lost or wrong, and the value model differs from `gram-value`'s. A tool that must be strict about gram (a loader, a linter, a formatter, a conformance runner that wants to catch a malformed case file) needs the syntax tree, which is why relate-gram reads tree-sitter-gram directly. That leaves the ecosystem without a library that reads gram faithfully into `gram-value` values with positions and diagnostics. Whether to build one, and where, is a decision for a separate RFC.

## 6. Reproducing

- Rust: `cd rust && cargo run -q -- ../inputs`
- TypeScript: `cd ts && npm install && node probe.mjs ../inputs`
- Python (3.11 to 3.13): `cd py && python -m venv .venv && .venv/bin/pip install relateby-pattern && .venv/bin/python probe.py ../inputs`
- Reference: `gram lint --strict inputs/<file>`
