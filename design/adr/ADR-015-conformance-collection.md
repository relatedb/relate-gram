---
id: ADR-015
title: A Language-Neutral Conformance Collection
status: accepted
rfcs: [RFC-005]
created: 2026-11-15
---

# ADR-015: A Language-Neutral Conformance Collection

## 1. Context

relate-gram's behavior is defined by its Rust tests. The invalid cases are files: 71 `.expect` files under `tests/fixtures/invalid/`, each beside a gram file, give the expected error code, line, column and byte offset. Everything else is code. The 25 valid fixtures are loaded by tests that assert the resulting graph inline, expected export text is written as string literals in the contract tests, the fixer's inputs and outputs are literals in `tests/fix.rs`, and the starting state of a database (the `Existing` nodes that every rejected file must leave untouched) is a Cypher statement in `tests/positions/main.rs`. Graphs are compared by a helper that dumps a database as sorted text lines. A second implementation, in Python or TypeScript, has to extract all of that before it can tell whether it agrees.

Four properties of the behavior shape any shared form:

- Positions are bytes. A diagnostic's `column` counts bytes from the start of the line, and its offset is a byte offset (ADR-007), so the text under test must be kept as exact bytes, and an implementation whose strings are UTF-16 or code points has to convert.
- The expected results are graphs and records, which gram writes natively. Relate's value types appear in them: integers and decimals that stay distinct, dates, points, UUIDs and, through the `embedding` tag, vectors. Gram's literals and tags already express these, and the full grammar adds a top-level record that can hold arrays and shallow maps, doc comments (`/** … */`), and annotations (`@key(value)`, whose value may be any value).
- relate-gram loads a strict subset of that grammar. It rejects annotations (`construct.annotation`), maps (`value.map`) and a record `kind` other than `property-graph`, so a file that uses them is not loadable by relate-gram. A collection file is read by a test runner, never loaded.
- A Relate package cannot always express a case. Relate's browser package has no explicit transactions and takes JSON parameters only, a package may lack a value type, and Relate refuses 16-bit vectors unless a process-wide environment flag is set.

`gram-value`, a library of gram's property values in five languages, runs against one corpus written in gram, read by each package with its own tree-sitter-gram binding, with a list of skipped cases per package and a parity table generated from the results.

Gram has libraries that read it into data. The pattern-rs project publishes `relateby-gram` for Rust, `@relateby/gram` for npm and `relateby.gram` for Python (0.7.2). Each parses gram into `Pattern<Subject>` values, and each has a parse that returns the top-level record separately from the patterns (`parse_gram_with_header`, `parseWithHeader`, `parse_with_header`). A spike (SPIKE-006) ran them over 34 small inputs. They read the shape of a case file correctly: a leading doc comment, a header record holding a map and an array, labeled nodes, tagged strings, integers and decimals kept apart, identifiers and paths. They are not a faithful reader of gram:

- repeated keys are merged silently and a fenced tag is dropped silently, where the grammar's linter (`gram lint`) reports repeated keys as errors;
- an empty record `{}` is not read as a header, and an annotation's map value becomes a string;
- the npm package turns `1.0` into `1` and cannot read an integer above 2^53 (the whole file fails), and the Python distribution does not install on Python 3.14;
- no value has a position, and arrow kinds and `::` before a label are lost or rejected;
- their values are not `gram-value` values: a bare symbol is a separate kind, and a tagged string is raw text.

Some of this is by design: gram's data model, `Pattern<Subject>`, does not keep style. A consumer that needs positions or arrow kinds has to keep the syntax tree, which is why relate-gram reads tree-sitter-gram directly (ADR-007). No library reads gram faithfully into `gram-value` values with positions and diagnostics. A prototype of one (SPIKE-007) closed these gaps and rebuilt 67 of the 71 invalid fixtures as a policy over it. The ecosystem direction, recorded in a design note in the new `gram-frame` repository that is not yet a decision, is a semantic layer over the tree-sitter-gram syntax tree that produces diagnostics and a flat table of classified elements, the frame. That layer does not exist yet, and a conformance collection should not wait for it: reading expectation files needs only exact values and a loud failure on a malformed file.

## 2. Decision

### 2.1 One collection, in this repository

The cases live in a `conformance/` directory in this repository. `conformance/VERSION` holds the collection's version, starting at 0.1.0, and `conformance/GRAMMAR_VERSION` pins the tree-sitter-gram and `gram` tool versions used to check the files. A release of the collection is a git tag `conformance-vMAJOR.MINOR.PATCH` in this repository. An implementation in another repository takes a copy at a tag, as a git submodule or a vendored directory. The Rust tests read the directory directly. A case that states a behavior change lands in the same pull request as the Rust change that makes it true, and it must fail against the code before that change.

### 2.2 A case is a directory, and its description is gram

A case is a directory `conformance/<kind>/<id>/`. The identifier is the directory name, is stable, and is never reused. The directory holds:

- `case.gram`, the description of the case, in gram. Its top-level record has `requires`, a list of capabilities (2.6, empty if none), and may have `settings`, a map, and `transaction: true`, which opens an explicit transaction on the connection before the operation runs. A doc comment explains why the case exists and cites the decision it tests; it is for readers of the file, and the runner does not read it. Each expected diagnostic, count or fixer result is a node: `(:Error {code: "rel.no-type", line: 2, column: 8, offset: 73})`, and likewise `Warning`, `Counts`, `Applied` and `Remaining`. An annotation decorates a pattern, and an expectation has no pattern to decorate, so expectations are nodes and annotations are not used.
- `input.txt`, the text under test, or `input.bin` for bytes that are not valid UTF-8. `exported.txt` and `fixed.txt` are the expected output text. These three are byte strings: the runner never parses them as gram, because a position is an offset into exact bytes and the text may use constructs that gram tooling would reject or rewrite. The collection's `.gitattributes` marks `conformance/**/*.txt` and `*.bin` as not text, so a checkout cannot change line endings or a byte order mark.
- `expected.gram`, the expected graph (2.3), in gram.
- `start.cypher`, where a case needs a starting database: Cypher statements, each ending in a semicolon at the end of a line, run in file order in a fresh database, with no parameters. A `reject` case uses the standard starting graph, kept once at `conformance/start.cypher`, unless it has its own. Running in file order gives every package the same internal order and so the same generated identifiers on export.

`gram lint --strict` runs over `case.gram` and `expected.gram`, and a check fails on a node label or capability name that is not in `conformance/vocabulary.gram`, so a misspelled `requires` entry cannot silently skip a case. The kinds are:

- **`reject`**: a text that must fail. The expected diagnostics are the case's `Error` and `Warning` nodes, with `code`, `line`, `column` and `offset`, compared exhaustively (2.4); a diagnostic with no position has `code` only. The runner loads the text into the starting database and requires that graph to be unchanged afterwards. It also checks the text without a database and requires the same errors. A `Counts` node may give `total`, the number of errors found before the cap on those reported.
- **`load`**: a text that must load. `case.gram` has the `settings`, `Warning` nodes and a `Counts` node; `expected.gram` is the graph afterwards; `start.cypher` is optional and defaults to an empty database. A reload case, one that tests matching against a database that already has elements, is a `load` case whose `start.cypher` creates the state an earlier load left. A `load` case may also have `exported.txt`, the text the resulting database exports to, which is also what the `normalize` command prints; the runner then loads that text into an empty database and requires an equal graph.
- **`export`**: a database made by `start.cypher`, the `settings`, and either `exported.txt` (compared byte for byte) or `Error` nodes naming the expected failures by code (2.5).
- **`fix`**: a text, the `settings` (these are the fixer's options), `fixed.txt`, `Applied` nodes with a `code` and a `count` for each correction, and `Remaining` nodes with the `code`, `line`, `column` and `offset` of each error left in `fixed.txt`.

`settings` keys mirror the library's own settings in lowerCamelCase. Load: `nodeIdProperty`, `relationshipIdProperty`, `tags` (`warn` or `error`), `maxDiagnostics`, `fileName`. Export: `identity` (`auto`, `generate` or `property`), `nodeIdProperty`, `relationshipIdProperty`. Fix: `relType`, `direction` (`right` or `left`), `octal` (`decimal` or `octal`), `annotate` (a boolean). A key that is absent means the library's default, and an unknown key fails the run. A `Counts` node uses `nodesCreated`, `nodesUpdated`, `relationshipsCreated`, `relationshipsUpdated`, `elementsWithoutIdentifier`, `referenceOnlyNodes`, `tagsDropped` and `total`.

### 2.3 Graphs are gram, and databases are made with Cypher

`expected.gram` is an ordinary gram file with these rules, which the runner enforces and fails on a breach. Every node is defined once, at top level, with its labels and properties. A relationship is written singly, with its end nodes named by identifier and each identifier defined at top level. There are no paths, forward references, annotations or maps. An identifier stands for the element's identity: the stored `@id` property, or the property that `nodeIdProperty` or `relationshipIdProperty` names. An element with no identity is written anonymous and cannot be an end of a relationship. The expected graph holds no maps or annotations, because Relate stores neither.

A runner reads `case.gram` and `expected.gram` with the pattern-rs gram library of its language, which returns the record and the patterns as data. The implementations under test read gram through tree-sitter-gram directly (ADR-007), so the files that state the expectations and the code that is checked against them take different parse paths, and a fault in one does not hide behind the same fault in the other. Once the semantic layer exists, runners can switch to its reader; the case files are gram, so no case changes.

Both sides of a comparison reduce to the same form without the exporter under test. A property value becomes a gram-value `Value` through one table kept in the collection's documentation: on the expected side the library's values map to `Value` (integer to integer, decimal to decimal, a bare symbol and a string to string, a tagged string to a tagged value, an array to an array), and on the database side the runner reads each property with Cypher's `valueType()` and `toString()` and maps that pair by the same table. The canonical text of the `Value` is then compared. A node is its sorted labels and its properties by sorted key; a relationship is its type, its properties and the lines of the nodes at its ends. Two graphs are equal when they have the same multiset of node lines and the same multiset of relationship lines. An element whose identifier was generated by the exporter is compared without its `@id`, so a reload of an export, whose anonymous elements get `n0` and `r0`, still equals the original.

This is weaker than isomorphism: two nodes with identical content are interchangeable, so a graph that swaps the relationships of two such nodes compares equal. It is the comparison the Rust tests make in spirit, with a dump of sorted lines, and it does not depend on any internal identifier or on order.

A starting database, and the database of an `export` case, is made by running `start.cypher` through the Relate package under test. Building it with the loader would let a loader fault change the starting state of every case it is used in, and Cypher can create states that gram cannot write, such as an integer vector.

### 2.4 What a case compares, and what it does not

A case compares the diagnostics (codes, lines, columns and byte offsets of every error and warning, exhaustively), the loaded graph, the counts it lists (a key that is absent is not checked), exported text byte for byte, and the fixer's output text and results. It does not compare message wording, which is written for people and may follow each language's idiom, nor the related positions a diagnostic carries. A diagnostic's `column` and `offset` count bytes of the UTF-8 text, as the Rust implementation does.

### 2.5 Export failures are compared by code

An export failure names an element by an internal identifier and a property key, and the identifier differs between databases. An `export` case lists the expected codes as `Error` nodes, and the runner compares them as a multiset. A case that must show which key is named checks that the message contains it only in the implementation's own tests.

### 2.6 Capabilities decide which cases apply

`requires` draws on a fixed vocabulary kept in `conformance/vocabulary.gram`. Each name has a probe, kept in `conformance/capabilities.gram`, that the runner applies at start-up:

- **Relate package:** `explicit-transactions` (begin then rollback succeeds), `temporal-values`, `points` and `uuids` (their constructor functions succeed), `vectors` (`vector([1.0], 1, FLOAT32)` succeeds) and `vector-16-bit` (the same with `FLOAT16`).
- **Gram reader:** `numeric-kinds` (the reader returns different kinds for `1` and `1.0`, which the pattern-rs npm package does not).
- **Operations:** `check`, `load`, `export` and `fix`, which an implementation declares. A kind implies the operations it runs: `reject` runs `check` and `load`, `load` runs `load` and, with `exported.txt`, `export`, `export` runs `export`, and `fix` runs `fix`.

A name in `requires` may start with `!` to require the capability to be absent. The 16-bit probe depends on a process-wide environment flag, which a runner sets before its first connection. A run therefore has two passes where it covers both states: cases that require `vector-16-bit` run with the flag set, and cases that require `!vector-16-bit` run with it unset. A declared `capabilities` file may only remove a capability, with a reason, never add one.

A case whose requirements are not met is not run and is reported `not-applicable`. A case that an implementation fails for any other reason is listed in its `known-failures.gram`, as `(:KnownFailure {case, reason, issue})`. It still runs and must fail; an unexpected pass fails the build. Each run writes `conformance-results.gram` with one `(:Result {case, status, version})` per case, where `status` is `passed`, `failed`, `skipped` (a known failure) or `not-applicable`, and `version` is the collection version. This has the same shape as `gram-value`'s results, so one script can build the parity table for both. An implementation's documentation of skipped cases is generated from that file.

### 2.7 Versions

`conformance/VERSION` is `MAJOR.MINOR.PATCH`. A new case raises the minor, a corrected description or a clarified file raises the patch, and a changed expected outcome raises the major. An implementation records the version it passes in its results and is free to lag; the results show how far. Whether an implementation's own version number tracks the Rust crate's is not decided here.

### 2.8 The Rust tests move onto it

A runner for the Rust library (`tests/conformance.rs`) reads the collection. The `invalid` fixtures and their `.expect` files become `reject` cases, the valid fixtures and the four reload fixtures become `load` cases, and the golden export strings and the fixer's literal examples become `export` and `fix` cases. A check requires every diagnostic code of the library to have a case, as the Rust coverage test does today; the table of codes stays in `docs/supported-gram.md`. These stay as Rust tests:

- tests of the library's internals and its public Rust types, property tests, performance tests, the command-line argument tests, the renderer, and the preview-flag test binaries;
- the order in which elements are created, because the equality of 2.3 ignores order;
- message contents, the related positions of a diagnostic, and the fixer's idempotence, its refusal of a fix that makes things worse, and its round and annotation counters;
- states that Cypher without parameters cannot create: NaN and infinities, the extreme integers, a NUL in a string, an empty key.

### 2.9 The collection tests relate-gram, and gram's own semantics belong to the semantic layer

The collection states what relate-gram accepts, rejects and writes. What gram itself means (what a repeated key is, what a duplicate definition is, how a multi-hop path lowers, how a forward reference resolves) is not specified anywhere today: tree-sitter-gram's reference lists these as open issues. The semantic layer in the `gram-frame` repository is to specify them, with its own corpus of gram text to frame and diagnostics. A case here that depends on such a rule, such as `id.duplicate-definition`, records relate-gram's behavior, and is reviewed against that specification when it exists. The general content forms of the known tags belong to `gram-value`'s corpus; relate-gram's `value.tag-content` cases cover only the Relate profile, which narrows them (a point needs its SRID, a time needs an offset). Relate's strict subset stays relate-gram's policy, and relate-gram keeps its own loader (ADR-007) whatever the layer provides.

### 2.10 The runner's parser dependency

This ADR is the ADR that the constitution's Rust Constraints require for a second parser. `relateby-gram` is a dev-dependency of the crate, used only by `tests/conformance.rs` to read the collection; the loader does not use it, and it resolves beside `relate`, `tree-sitter` 0.25 and `gram-value`. On acceptance the first bullet of the Rust Constraints gains "and, in tests, on `relateby-gram` to read the conformance collection", which raises the constitution by a minor version.

## 3. Alternatives Considered

3.1. **JSON case files.** Rejected: gram is the notation this tool exists for, and it already carries what the collection needs, so JSON would add a second encoding for each Relate value type (a `float` key, a `date` key, a `vector` key) beside gram's own literals and tags. Gram has libraries in all three first languages that return a file's record and patterns as data, so reading gram is a library call, as reading JSON would be. The cost is one dependency per language (the pattern-rs package), which JSON does not have. The implementations read gram through tree-sitter-gram and the runner through the pattern-rs libraries, so the parse paths differ; the text under test stays a separate file read as bytes.

3.2. **One TOML manifest.** Rejected: Node.js has no TOML reader in its standard library, and the manifest would hold what gram already holds.

3.3. **A repository of its own.** Rejected: a behavior change would need two coordinated pull requests, and today every change originates in this repository. It can move when the collection has owners besides this crate.

3.4. **Inside the `gram-value` corpus.** Rejected: that corpus is about values and needs no database, and its cases and runners are not about loading, rejecting or exporting.

3.5. **Use exported gram text as the expected graph.** Rejected: the expected graph would then come out of the exporter under test, and a fault in the loader and a fault in the exporter could cancel. `expected.gram` is written by hand and read by the runner, not produced by the implementation.

3.6. **Compare messages.** Rejected: the wording is for people and varies with each language's idiom, and a change of phrasing would fail every implementation.

3.7. **Declare capabilities by hand only.** Rejected: whether a Relate package can express a case is a property of the package and its environment, and a declared file can drift from it. Probes at start-up state it once, and the declared file may only remove.

3.8. **Annotations for the expectations.** Rejected: an annotation decorates a pattern, and an expectation such as an error at a position has no pattern in the case file to decorate, so an anchor node would have to be invented. A node per expectation is plain gram and needs no anchor.

3.9. **Build starting graphs by loading gram.** Rejected: a fault in the loader would change the starting state of every case that uses it, and gram cannot write every state a database can hold.

3.10. **Wait for the semantic layer before building the collection.** Rejected: reading expectation files needs only exact values and a loud failure on a malformed file, which the existing libraries plus the linter give today (SPIKE-006), and the case files are gram, so they do not change when the reader does.

3.11. **Write expected graphs as frames now.** Rejected for now: the frame is not specified yet, and its treatment of anonymous elements is open. The collection does not depend on it, and a runner reading frames later would need only a new reader for `expected.gram`.

3.12. **Read the case files with each language's tree-sitter-gram binding, as `gram-value` does.** Not chosen: each runner would write its own lowering of the syntax tree to typed values, and the runner and the implementations would share the grammar's parse path, so a fault in it could hide. In its favor, the values come out typed and with the grammar's exact reading, which removes the pattern-rs limits (TypeScript `1.0`, the 2^53 limit, Python below 3.14) and the conversion table on the expected side. It remains the fallback if the pattern-rs libraries prove too lossy.

3.13. **One gram file per topic, with the cases as nodes, as `gram-value` does.** Rejected: the text under test must be exact bytes, which a gram string inside a node cannot guarantee, and one directory per case keeps each case's review and conflicts local. The cost is several hundred small files.

## 4. Consequences

- Expected graphs for the valid fixtures and the golden exports have to be written out as gram; the rejected cases convert mechanically. The Rust tests that assert the same content inline are replaced by the runner, so the checking is not duplicated.
- Each runner is real code: a gram-to-graph reading of `expected.gram` under the rules of 2.3, the value table on both sides, the database dump through Cypher, the capability probes and the results file. These, more than the Rust runner, are the work of the first spec.
- The collection is several hundred small files, which is a cost in review and merge conflicts, and the Rust tests lose the creation-order assertions and the other checks that 2.8 leaves in Rust.
- Each runner depends on the pattern-rs gram library of its language (`relateby-gram`, `@relateby/gram`, `relateby.gram`), and the collection's CI runs `gram lint --strict` over `case.gram` and `expected.gram`. The npm package is backed by WebAssembly and has to be initialized before use, and the Python package needs Python 3.13 or older.
- A runner reads gram with two different libraries over time: the pattern-rs ones now, and the semantic layer's when it exists. Cases that depend on distinguishing integers from decimals need `numeric-kinds`, which the TypeScript runner lacks today.
- A collection file is not loadable by relate-gram (`file.kind`, `value.map`), and `relate-gram check` reports errors on a `case.gram`. The collection's documentation says so, and `gram lint --strict` is the check that applies.
- An implementation whose strings are not UTF-8 has to convert positions to bytes (2.4).
- The equality is content-based and weaker than isomorphism (2.3). A case that needs more would need a different comparison.
- Export failures are compared by code only, so a mistaken element or key in an export message is not caught by the collection.
- Integer vectors cannot appear in an expected graph, because gram has no tag for them; they are tested through the `export` kind, whose starting database is Cypher.
- RFC-006's graph model and this equality are two definitions of graph equality over the same subset; RFC-006 should reuse this one, or the reverse.
- Follow-up work: a spec for the extraction and the runners; the standard starting graph, the value table, the vocabulary and capability probes, and the results format; the table generation across implementations kept in other repositories. RFC-005 question 7.6 is half answered (the collection version an implementation passes is recorded); whether implementation version numbers track the Rust crate's, and question 7.7, remain open.

## 5. Related

- RFC-005 (resolves questions 7.4 and 7.5, and the recording of skipped cases in 3.5).
- ADR-007 (positions are bytes; the loader's parser), ADR-006 and ADR-011 to ADR-014 (the values an expected graph carries), ADR-010 (the fixer).
- SPIKE-006 (the pattern-rs readers) and SPIKE-007 (a typed syntax layer over tree-sitter-gram), in `design/spikes/`; the note `design/notes/2026-11-16-layers-and-the-frame.md` in the `gram-frame` repository (the layered design: syntactic parser, semantic parser, frame), which is freeform and non-binding.
- `gram-value`'s conformance corpus and its ADR-001 (the precedent for a shared corpus written in gram, per-package skips and a generated parity table, and for a linted vocabulary and a pinned grammar version).
