---
id: ADR-015
title: A Language-Neutral Conformance Collection
status: proposed
rfcs: [RFC-005]
created: 2026-11-15
---

# ADR-015: A Language-Neutral Conformance Collection

## 1. Context

relate-gram's behavior is defined by its Rust tests. The invalid cases are files: 71 `.expect` files under `tests/fixtures/invalid/`, each beside a gram file, give the expected error code, line, column and byte offset. Everything else is code. The 25 valid fixtures are loaded by tests that assert the resulting graph inline, expected export text is written as string literals in the contract tests, the fixer's inputs and outputs are literals in `tests/fix.rs`, and the starting state of a database (the `Existing` nodes that every rejected file must leave untouched) is a Cypher statement in `tests/positions/main.rs`. A second implementation, in Python or TypeScript, has to extract all of that before it can tell whether it agrees.

Four properties of the behavior shape any shared form:

- Positions are bytes. A diagnostic's `column` counts bytes from the start of the line, and its offset is a byte offset (ADR-007), so the text under test must be kept as exact bytes, and an implementation whose strings are UTF-16 or code points has to convert.
- The expected results are graphs and records, which gram writes natively. Relate's value types appear in them: integers and decimals that stay distinct, dates, points, UUIDs and, since `embedding`, vectors. Gram's literals and tags already express these, and the full grammar adds a top-level record that can hold arrays and shallow maps, doc comments (`/** … */`), and annotations (`@key(value)`, whose value may be any value).
- relate-gram loads a strict subset of that grammar. It rejects annotations (`construct.annotation`), maps (`value.map`) and a record `kind` other than `property-graph`, so a file that uses them is not loadable by relate-gram. A collection file is read by a test runner, never loaded.
- A Relate package cannot always express a case. Relate's browser package has no explicit transactions and takes JSON parameters only, and a package may lack a value type.

`gram-value` runs its languages against one corpus written in gram, read by each package with its own tree-sitter-gram binding, with a list of skipped cases per package and a parity table generated from the results.

Gram has libraries that read it into data. The pattern-rs project publishes `relateby-gram` for Rust, `@relateby/gram` for npm and `relateby.gram` for Python. Each parses gram into `Pattern<Subject>` values, and each has a parse that returns the top-level record separately from the patterns (`parse_gram_with_header`, `parseWithHeader`, `parse_with_header`). A subject's property values keep integers, decimals, tagged strings, arrays and maps. ADR-007 declined this path for the loader, because it drops what the loader must check (positions, repeated keys, integers beyond 64 bits). A runner that reads hand-written case files needs none of that.

## 2. Decision

### 2.1 One collection, in this repository

The cases live in a `conformance/` directory in this repository, with a `conformance/VERSION` file. The Rust tests read it directly. An implementation in another repository takes a pinned copy, a git submodule or a vendored directory, at a released version of the collection. Behavior changes land here first, together with the Rust change that makes them true.

### 2.2 A case is a directory, and its description is gram

A case is a directory `conformance/<kind>/<id>/`. The identifier is the directory name, is stable, and is never reused. The directory holds:

- `case.gram`, the description of the case, in gram. Its top-level record gives the `kind`, the `requires` list of capabilities (2.6) and the case's settings, with maps and arrays as the grammar allows. A doc comment explains why the case exists and cites the decision it tests; it is for readers of the file, and the runner does not read it. Each expected diagnostic, warning or count is a node: `(:Error {code: "rel.no-type", line: 2, column: 8, offset: 73})`. An annotation decorates a pattern, and an expectation has no pattern to decorate, so expectations are nodes and annotations are not used.
- `input.gram`, the text under test, kept as exact bytes in its own file, since a position is an offset into those bytes and the text may itself use constructs that a case file would not.
- `start.cypher`, where a case needs a starting database, and `expected.gram` or `exported.gram` where it needs an expected graph or expected text (below).

The collection is checked with `gram lint --strict`. The kinds are:

- **`reject`**: a text that must fail. `case.gram` holds one `Error` node per expected error with `code`, `line`, `column` and `offset`, and nothing else may be reported as an error. The runner loads the text into a database made by `start.cypher`, which is the standard starting graph kept once at `conformance/start.cypher` unless the case has its own, and requires that graph to be unchanged afterwards. It also checks the text without a database and requires the same errors.
- **`load`**: a text that must load. `case.gram` holds the settings, `Warning` nodes for the expected warnings, and a `Counts` node. `expected.gram` is the expected graph. A `load` case may also have `exported.gram`, the text the resulting database exports to; the runner then loads that text into an empty database and requires an equal graph.
- **`export`**: a database made by `start.cypher`, the settings, and either `exported.gram` (the expected text, compared byte for byte) or `Error` nodes naming the expected failures by code (2.5).
- **`fix`**: a text, the options in the record, `fixed.gram` (the expected output text), `Applied` nodes with a `code` and a `count` for each correction, and `Remaining` nodes with the codes of the errors that remain.

### 2.3 Graphs are gram, and databases are made with Cypher

An expected graph is an ordinary gram file. A runner reads `case.gram`, `expected.gram`, `exported.gram` and `fixed.gram` with the pattern-rs gram library of its language, which returns the record and the patterns as data, so the types come from gram itself: an integer or a decimal literal, a tagged string for a date, point, UUID or embedding. The implementations under test read gram through tree-sitter-gram directly (ADR-007), so the files that state the expectations and the code that is checked against them take different parse paths, and a fault in one does not hide behind the same fault in the other. An identifier written in `expected.gram` stands for the stored `@id` property, as the loader stores it, and an element without an identifier has none. The expected graph holds no maps or annotations, because Relate stores neither.

Two graphs are equal when they have the same multiset of nodes and the same multiset of relationships, where a node is its sorted labels and properties, and a relationship is its type, its properties and the nodes at its ends. Property values compare by gram-value's canonical text: the runner maps the codec's values to gram values on the expected side, and a database's values as the exporter does on the other. The comparison depends on no internal identifier and not on order. It is the comparison the Rust tests make today with a database dump.

A starting database, and the database of an `export` case, is made by running `start.cypher` through the Relate package under test. Building it with the loader would let a loader fault change the starting state of every case it is used in, and Cypher can create states that gram cannot write, such as an integer vector.

### 2.4 What a case compares, and what it does not

A case compares codes, severities, lines, columns and byte offsets, the loaded graph, warnings by code and position, counts, exported text byte for byte, and the fixer's output text. It does not compare message wording, which is written for people and may follow each language's idiom. A diagnostic's `column` and `offset` count bytes of the UTF-8 text, as the Rust implementation does.

### 2.5 Export failures are compared by code

An export failure names an element by an internal identifier and a property key, and the identifier differs between databases. An `export` case lists the expected codes as `Error` nodes, and the runner compares them as a multiset. A case that must show which key is named checks that the message contains it only in the implementation's own tests.

### 2.6 Capabilities decide skips, and known failures are separate

`requires` draws on a fixed vocabulary kept in the collection: `explicit-transactions`, `vectors`, `vector-16-bit`, `typed-parameters`, `in-memory`. Each implementation states the capabilities of its Relate package in a small file. The runner skips a case whose requirements are not met and reports it as skipped with the missing capability, with no hand-written list. A case that an implementation fails for any other reason goes in that implementation's `known-failures` file with a reason and an issue; the runner reports it separately and treats an unexpected pass as an error. Each run writes a `results` file recording, for every case, passed, failed, skipped or known-failing, and the collection version.

### 2.7 Versions

`conformance/VERSION` is `MAJOR.MINOR.PATCH`. A new case raises the minor, a corrected description or a clarified file raises the patch, and a changed expected outcome raises the major. A behavior change starts with the case, and the Rust implementation fails it until the code changes. An implementation records the version it passes in its results. Implementations are free to lag, and the results show how far.

### 2.8 The Rust tests move onto it

A runner for the Rust library (`tests/conformance.rs`) reads the collection. The `invalid` fixtures and their `.expect` files become `reject` cases, the valid fixtures become `load` cases with expected graphs written out, and the golden export strings and the fixer's literal examples become `export` and `fix` cases. Tests about the library's internals, property tests, performance tests, the command-line argument tests and the tests of the preview flag stay as Rust tests.

## 3. Alternatives Considered

3.1. **JSON case files.** Rejected: gram is the notation this tool exists for, and it already carries what the collection needs, so JSON would add a second encoding for each Relate value type (a `float` key, a `date` key, a `vector` key) beside gram's own literals and tags. Gram has libraries in all three first languages that return a file's record and patterns as data, so reading gram is a library call, as reading JSON would be. The cost is one dependency per language (the pattern-rs package), which JSON does not have. A first draft chose JSON out of caution that a runner reading gram would share a parser with the code under test. The implementations read gram through tree-sitter-gram, and the runner through the pattern-rs libraries, so the paths differ; the text under test stays a separate file read as bytes.

3.2. **One TOML manifest.** Rejected: Node.js has no TOML reader in its standard library, and the manifest would hold what gram already holds.

3.3. **A repository of its own.** Rejected: a behavior change would need two coordinated pull requests, and today every change originates in this repository. It can move when the collection has owners besides this crate.

3.4. **Inside the `gram-value` corpus.** Rejected: that corpus is about values and needs no database, and its cases and runners are not about loading, rejecting or exporting.

3.5. **Use exported gram text as the expected graph.** Rejected: the expected graph would then come out of the exporter under test, and a fault in the loader and a fault in the exporter could cancel. `expected.gram` is written by hand and read by the runner, not produced by the implementation.

3.6. **Compare messages.** Rejected: the wording is for people and varies with each language's idiom, and a change of phrasing would fail every implementation.

3.7. **Hand-written skip lists only.** Rejected: whether a Relate package can express a case is a property of the package, and stating it once as capabilities keeps every skip explained by the same vocabulary. A list is still needed for real failures, which is the known-failures file.

3.8. **Annotations for the expectations.** Rejected: an annotation decorates a pattern, and an expectation such as an error at a position has no pattern in the case file to decorate, so an anchor node would have to be invented. A node per expectation is plain gram and needs no anchor.

3.9. **Build starting graphs by loading gram.** Rejected: a fault in the loader would change the starting state of every case that uses it, and gram cannot write every state a database can hold.

## 4. Consequences

- Expected graphs for the valid fixtures and the golden exports have to be written out as gram; the rejected cases convert mechanically. The Rust tests that assert the same content inline are replaced by the runner, so the checking is not duplicated.
- Each runner depends on the pattern-rs gram library of its language (`relateby-gram`, `@relateby/gram`, `relateby.gram`). In Rust it is a dev-dependency used only to read the collection; ADR-007 and the constitution's parser constraint concern the loader, and the spec's constitution check confirms that test-only use is covered. The npm package is backed by WebAssembly and has to be initialized before use.
- A collection file is not loadable by relate-gram (`file.kind`, `value.map`), and `relate-gram check` reports errors on a `case.gram`. The collection's documentation says so, and `gram lint --strict` is the check that applies.
- An implementation whose strings are not UTF-8 has to convert positions to bytes. This is the cost of positions that mean the same thing everywhere.
- The vocabulary of node labels in `case.gram` (`Error`, `Warning`, `Counts`, `Applied`, `Remaining`) and of capabilities must grow with the behavior, and the collection's documentation has to state both exactly.
- Export failures are compared by code only, so a mistaken element or key in an export message is not caught by the collection.
- Integer vectors cannot appear in an expected graph, because gram has no tag for them; they are tested through the `export` kind, whose starting database is Cypher.
- Follow-up work: a spec for the extraction and the Rust runner; the standard starting graph; the label and capability vocabularies and the results format; the table generation across implementations kept in other repositories. RFC-005 open questions 7.6 and 7.7 (versions of the implementations, and which operations come first) remain open.

## 5. Related

- RFC-005 (resolves questions 7.4 and 7.5, and the recording of skipped cases in 3.5).
- ADR-007 (positions are bytes), ADR-006 and ADR-011 to ADR-014 (the values an expected graph carries), ADR-010 (the fixer).
- `gram-value`'s conformance corpus and its ADR-001 (the precedent for a shared corpus written in gram, per-package skips and a generated parity table).
