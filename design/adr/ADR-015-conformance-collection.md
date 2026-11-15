---
id: ADR-015
title: A Language-Neutral Conformance Collection
status: proposed
rfcs: [RFC-005]
created: 2026-11-15
---

# ADR-015: A Language-Neutral Conformance Collection

## 1. Context

relate-gram's behavior is defined by its Rust tests. The invalid cases are files: 71 `.expect` files under `tests/fixtures/invalid/`, each beside a gram file, give the expected error code, line, column and byte offset. Everything else is code. The 29 valid fixtures are loaded by tests that assert the resulting graph inline, expected export text is written as string literals in the contract tests, the fixer's inputs and outputs are literals in `tests/fixtures`-less tests in `tests/fix.rs`, and the starting state of a database (the `Existing` nodes that every rejected file must leave untouched) is a Cypher statement in `tests/positions/main.rs`. A second implementation, in Python or TypeScript, has to extract all of that before it can tell whether it agrees.

Four properties of the behavior shape any shared form:

- Positions are bytes. A diagnostic's `column` counts bytes from the start of the line, and its offset is a byte offset (ADR-007), so an implementation whose strings are UTF-16 or code points has to convert.
- The cases are about gram text. A reader of the collection that parses gram with the parser under test would hide that parser's faults from the very cases meant to catch them.
- Expected graphs hold Relate's value types: integers and floats that must stay distinct, dates, points, UUIDs and, since `embedding`, vectors. Plain JSON cannot tell `1` from `1.0`, nor a date from a string.
- A Relate package cannot always express a case. Relate's browser package has no explicit transactions and takes JSON parameters only, and a package may lack a value type.

`gram-value` runs its languages against one corpus, with a list of skipped cases per package and a parity table generated from the results. Its corpus is written in gram, and describes values without a database.

## 2. Decision

### 2.1 One collection, in this repository

The cases live in a `conformance/` directory in this repository, with a `conformance/VERSION` file. The Rust tests read it directly. An implementation in another repository takes a pinned copy, a git submodule or a vendored directory, at a released version of the collection. Behavior changes land here first, together with the Rust change that makes them true.

### 2.2 A case is a directory of plain files

A case is a directory `conformance/<kind>/<id>/`. Its `case.json` states the case, and the gram text, expected text or other bulky inputs are files beside it. The identifier is the directory name, is stable, and is never reused. JSON is the format because every target language reads it without a library. The kinds are:

- **`reject`**: a gram text that must fail. `case.json` lists the expected errors, each with `code`, `line`, `column` and `offset`, and nothing else may be reported as an error. The runner loads the text into a database that holds the standard starting graph (two nodes and a relationship, defined once in the collection) and requires that graph to be unchanged afterwards. It also checks the text without a database and requires the same errors.
- **`load`**: a gram text that must load. `case.json` gives the settings (default: none), an optional starting graph (default: empty), and the expected graph, the expected warnings (code and position) and the expected counts. A `load` case may also give the text that the resulting database exports to. The runner then loads that text into an empty database and requires an equal graph.
- **`export`**: a graph, settings, and either the expected text or the expected failures. An export failure is compared by its multiset of codes (2.5).
- **`fix`**: a gram text, the options, the expected output text, the counts of applied corrections by code, and the codes that remain.

### 2.3 Graphs are described in JSON with explicit value types

A graph is a list of nodes and a list of relationships. A node has `labels` and `properties`; a relationship has `type`, `from`, `to` and `properties`, where `from` and `to` name a node by its position in the list. A property value is a JSON string (a string), a JSON integer (an integer), a JSON boolean (a boolean), or an object with one key that names the type: `float`, `date`, `time`, `localtime`, `datetime`, `localdatetime`, `duration`, `point`, `uuid`, `vector`, or `list`. The `float` and temporal keys hold the same text a gram file would, `vector` holds the vector kind and its elements, and `list` holds an array of values. A float is written as text, so `1` and `1.0` cannot be confused.

Two graphs are equal when they have the same multiset of nodes and the same multiset of relationships, where a node is its sorted labels and properties, and a relationship is its type, its properties and the nodes at its ends. Property values compare by gram-value's canonical text, which every language package already has. The comparison depends on no internal identifier and not on order. It is the comparison the Rust tests make today with a database dump.

### 2.4 What a case compares, and what it does not

A case compares codes, severities, lines, columns and byte offsets, the loaded graph, warnings by code and position, counts, exported text byte for byte, and the fixer's output text. It does not compare message wording, which is written for people and may follow each language's idiom. A diagnostic's `column` and `offset` count bytes of the UTF-8 text, as the Rust implementation does.

### 2.5 Export failures are compared by code

An export failure names an element by an internal identifier and a property key, and the identifier differs between databases. The `export` case lists the expected codes, in any order, and the runner compares them as a multiset. A case that must show which key is named includes the key in the failing graph's description and checks that the message contains it only in the implementation's own tests.

### 2.6 Capabilities decide skips, and known failures are separate

A case may list `requires`, drawn from a fixed vocabulary kept in the collection: `explicit-transactions`, `vectors`, `vector-16-bit`, `typed-parameters`, `in-memory`. Each implementation states the capabilities of its Relate package in a small file. The runner skips a case whose requirements are not met and reports it as skipped with the missing capability, with no hand-written list. A case that an implementation fails for any other reason goes in that implementation's `known-failures` file with a reason and an issue, and the runner reports it separately and treats an unexpected pass as an error. Each run writes a `results` file recording, for every case, passed, failed, skipped or known-failing, and the collection version.

### 2.7 Versions

`conformance/VERSION` is `MAJOR.MINOR.PATCH`. A new case raises the minor, a corrected description or a clarified file raises the patch, and a changed expected outcome raises the major. A behavior change starts with the case, and the Rust implementation fails it until the code changes. An implementation records the version it passes in its results. Implementations are free to lag, and the results show how far.

### 2.8 The Rust tests move onto it

A runner for the Rust library (`tests/conformance.rs`) reads the collection. The `invalid` fixtures and their `.expect` files become `reject` cases, the valid fixtures become `load` cases with expected graphs written out, and the golden export strings and the fixer's literal examples become `export` and `fix` cases. Tests about the library's internals, property tests, performance tests, the command-line argument tests and the tests of the preview flag stay as Rust tests.

## 3. Alternatives Considered

3.1. **Write the cases in gram, as `gram-value` does.** Rejected: the collection's input is gram, so reading it with a gram parser makes the parser under test part of the harness, and a parser fault could hide a case. Expected graphs of Relate's value types need an encoding of their own in either form.

3.2. **One TOML manifest.** Rejected: Node.js has no TOML reader in its standard library, and JSON is read natively by Rust, Python and TypeScript.

3.3. **A repository of its own.** Rejected: a behavior change would need two coordinated pull requests, and today every change originates in this repository. It can move when the collection has owners besides this crate.

3.4. **Inside the `gram-value` corpus.** Rejected: that corpus is about values and needs no database, and its cases and runners are not about loading, rejecting or exporting.

3.5. **Use exported gram text as the expected graph.** Rejected: a fault in the loader and a fault in the exporter could cancel, and the case would pass.

3.6. **Compare messages.** Rejected: the wording is for people and varies with each language's idiom, and a change of phrasing would fail every implementation.

3.7. **Hand-written skip lists only.** Rejected: whether a Relate package can express a case is a property of the package, and stating it once as capabilities keeps every skip explained by the same vocabulary. A list is still needed for real failures, which is the known-failures file.

## 4. Consequences

- Expected graphs for the valid fixtures and the golden exports have to be written out; the rejected cases convert mechanically. The Rust tests that assert the same content inline are replaced by the runner, so the checking is not duplicated.
- An implementation whose strings are not UTF-8 has to convert positions to bytes. This is the cost of positions that mean the same thing everywhere.
- The graph encoding must grow with Relate's value types. A new type needs a key in 2.3, and implementations that cannot express it declare the capability.
- Export failures are compared by code only, so a mistaken element or key in an export message is not caught by the collection.
- The capability vocabulary, the results format and the table of results per implementation are conventions that the collection's documentation has to state exactly, and a script has to merge results from implementations kept in other repositories.
- Follow-up work: a spec for the extraction and the Rust runner; the standard starting graph; the vocabulary and results formats; the table generation. RFC-005 open questions 7.6 and 7.7 (versions of the implementations, and which operations come first) remain open.

## 5. Related

- RFC-005 (resolves questions 7.4 and 7.5, and the recording of skipped cases in 3.5).
- ADR-007 (positions are bytes), ADR-006 and ADR-011 to ADR-014 (the values the graph description carries), ADR-010 (the fixer).
- `gram-value`'s conformance corpus and its ADR-001 (the precedent for a shared corpus, per-package skips and a generated parity table).
