---
id: RFC-005
title: relate-gram in Other Languages
status: draft
created: 2026-11-15
adrs: []
---

# RFC-005: relate-gram in Other Languages

## 1. Summary

Relate is an embedded Cypher graph database. It is used from Rust, Python, Node.js, the browser, and Go, Java and Swift. relate-gram is the tool that loads a gram file (a text notation for property graphs) into a Relate database and writes a database back out as gram. It is a Rust crate and a command, and nothing else, so a program in any other language cannot load or export gram without leaving its own connection behind.

The organizing claim is that what has to be shared across languages is behavior, not code: which files are accepted, which are rejected and with what error at what position, what graph a file loads to, and what text a database exports to. Behavior can be written down as cases that any implementation runs. Once it is, the choice between wrapping the Rust tool, binding it, or writing each language's version becomes a choice about cost and fit, and it can be revisited without losing correctness.

This RFC settles the problem, what an implementation in another language must deliver, the candidate directions with their trade-offs, and the questions that remain. It proposes Python and TypeScript as the first two languages.

## 2. Problem / Motivation

Relate ships a Python package (`relate-db`, version 1.0.2, built with PyO3), npm packages for Node.js and for the browser (WebAssembly), and clients for Go, Java and Swift. relate-gram is used through the Rust crate or through the `relate-gram` command, and the command takes a database directory, as in `relate-gram load FILE --db PATH`.

Three costs follow for someone outside Rust:

- A Python or JavaScript program that already holds a Relate connection, including an in-memory database, cannot hand it to relate-gram. The Rust crate takes a connection from the Rust `relate` crate, and a connection from the Python or npm package is a different object. The command works only on a database on disk, and a database that a program has open cannot also be opened by a second process.
- The only way to check a gram file without a database, to correct it, or to normalize it is the command. A program that wants the diagnostics as data has to parse command output.
- The behavior of the tool is already specified in detail and tested in Rust: about 170 fixture and expectation files under `tests/fixtures/` (valid files, and invalid files with the expected error code, line, column and byte offset) and golden strings for exported text in the contract tests. That specification is readable only through Rust tests, so a second implementation has no way to know whether it agrees.

The same gap exists for the shared value library `gram-value`, which is published for Rust, TypeScript, Python, Go and Swift and is tested against one corpus of cases that every language runs. relate-gram has no equivalent, so each new language would have to rediscover what the Rust code does by reading it.

## 3. Goals

3.1. A Python program can check, load and export gram with the connection it already has, including an in-memory database, and gets the same accepted and rejected files, error codes and positions, loaded graphs and exported text as the Rust crate.

3.2. A TypeScript program, in Node.js and in the browser where Relate's browser package allows it, gets the same.

3.3. Correct behavior is defined once, in cases that are independent of any implementation language, and every implementation, the Rust one included, runs them.

3.4. A change to behavior (a new rejection, a changed export spelling) starts as a failing case in that shared definition, and an implementation that disagrees fails.

3.5. An implementation that cannot offer part of the behavior (a Relate package that cannot store a value type, for example) says so in its documentation and in a located error, and does not approximate.

## 4. Non-Goals

4.1. New gram constructs or new Relate value types. The strict subset and the typed tags stay as they are.

4.2. Implementations for Go, Java and Swift. They are possible with the same corpus and are left for when someone needs them.

4.3. A `relate run` extension. The extension mechanism of the Relate command-line tool lets a command run as a child process that the host spawns, and relate-gram could be packaged that way in any language. It serves people using the Relate command, not programs using a library, and the choice does not block or replace anything here.

4.4. Bringing relate-gram into the Relate repository, or changing its Rust interface.

4.5. Matching the speed of the Rust implementation. Loading is dominated by statements sent to the database.

## 5. Proposed Approach (high-level)

### 5.1 What must be the same

Four things define an implementation's behavior, and each can be tested without knowing how it is built:

- The accept or reject decision for a gram text, and for a rejection the diagnostic: its code, severity, line, column and byte offset.
- The graph that an accepted text loads into an empty database: nodes, relationships, labels, types and property values with their types.
- The text a database exports to, byte for byte, including identifiers, quoting and the canonical spelling of values.
- The effect of a load on a non-empty database: which elements are matched by identifier and which are created.

Parsing, diagnostics and value spelling need no database. Loading and exporting reach the database only through Cypher statements with parameters, which every Relate package can issue.

### 5.2 A shared definition of correct behavior

The cases already exist as Rust fixtures and expectations. They can be collected in a form that is not tied to Rust: gram files, each with the expected outcome (loaded graph, or diagnostics with positions), databases described by their contents with the expected exported text, and the table of diagnostic codes. Every implementation reads the same collection and reports which cases it fails. The Rust crate's own tests would read it too, so the collection has no second owner.

This is the arrangement `gram-value` uses for its languages, and it is what keeps independent implementations from drifting. It is also the cheapest step: it needs no new code in any other language and it improves the Rust tests.

### 5.3 Candidate strategies

**5.3.1 Wrap the command.** A package for each language runs `relate-gram` as a subprocess and returns its output.
- Gives: no duplicated logic and exact agreement with the Rust tool.
- Costs: on-disk databases only, so no in-memory database and no connection the program already holds; a binary for each platform; diagnostics as parsed text.

**5.3.2 Bind the Rust crate.** Compile the Rust crate into a Python extension module and a Node.js addon.
- Gives: the same parser and the same code.
- Costs: the crate works on a connection from the Rust `relate` crate, and a Python or npm connection is another object, possibly from another build of the engine. Either the binding opens the database a second time, or the crate is changed to work through a small connection interface that the host language fills in with callbacks. The first risks two writers on one store. The second changes the crate's interface and puts a value conversion for every Relate type on the language boundary. Each language also needs a native build and a platform wheel or addon.

**5.3.3 Native implementations.** A package for each language written in that language, built on the language's gram-value package, the tree-sitter-gram binding for that language, and the language's Relate package, sharing nothing with the Rust crate except the cases of 5.2.
- Gives: it works with the connection the program has, including in-memory, with no native build of its own; diagnostics as ordinary values; the browser for TypeScript.
- Costs: the logic is written again for each language (the Rust crate is about 5,000 lines, of which the parser walk, the identity planning, the typed-tag profile, the loader and the exporter are the parts to port), and agreement depends on the cases.

**5.3.4 A `relate run` extension.** Described in 4.3; it is a packaging choice that a native implementation or the Rust crate could also use.

### 5.4 Proposed direction

Write down the shared cases first (5.2), then build native implementations (5.3.3), Python first and TypeScript next, each tested against the cases from the start. Wrapping the command (5.3.1) remains available as a stopgap for a program that can accept its limits and is not a substitute for either.

Python is first because its Relate package is mature and already has a vector type, which is the hardest of the value types to carry. TypeScript follows and adds the browser. The first implementation is also the way to learn what the cases leave out.

### 5.5 Cross-cutting concerns

- **Value types.** Each Relate package represents values differently, and an implementation can only store or return what its package can. Where a type is missing, the case for it is skipped with a stated reason (3.5), and the corpus records which implementations skip which cases.
- **Versions.** The implementations and the Rust crate describe the same behavior; whether they share version numbers is open (7.5).
- **Operations.** `check` needs no database and is the cheapest part to offer first; `fix` and `normalize` build on it. Which operations each implementation offers is open (7.6).
- **Distribution.** Native packages are plain source in each language, so they ship through each language's usual registry with no platform builds of their own.

## 6. Alternatives Considered

6.1. **Stay Rust-only and document the command.** Rejected: it leaves the costs of section 2 in place for every program outside Rust, and the Python and JavaScript packages of Relate are where most embedded use is likely to come from.

6.2. **One Rust core compiled to WebAssembly and loaded from every language.** Rejected: the crate needs a database connection on the other side of the boundary, so the connection problem of 5.3.2 returns; the parser is a C library that has to be built for WebAssembly; and `gram-value` measured plain-object TypeScript at 12 to 25 times the speed of a WebAssembly core for values, which is a small part of this work but sets the expectation for boundary cost.

6.3. **A generated implementation from a schema.** Not rejected: code generation for the diagnostics table, the tag profile and the case reader is likely to help in each language. It is a tool choice for the decisions that follow and does not change the direction.

## 7. Open Questions

7.1. Do Relate's Node.js and WebAssembly packages store and return vectors, and do they accept the statement forms the loader uses (parameter batches, a literal vector kind, the preview flag for 16-bit kinds)? The answer decides whether the TypeScript implementation can support the `embedding` tag.

7.2. Which language comes first? Python is proposed above for its package's maturity and vector support; TypeScript reaches the browser. The deciding evidence is how complete each Relate package's interface is for what the loader needs.

7.3. Where does the shared collection of cases live: in this repository, in a repository of its own, or alongside the `gram-value` corpus? It must have one owner and be readable from every language's test suite.

7.4. In what form is a case written, and how is a loaded graph or a database's contents compared? The existing tests compare database dumps in Rust and golden strings, and a neutral form must express both.

7.5. Do the implementations share version numbers with the Rust crate, or version independently against the behavior they pass?

7.6. Which operations does each implementation offer first: `check`, `load`, `export`, `normalize`, `fix`, and a command-line tool?

7.7. How are skipped cases reported (7.1 and 5.5), and when does a skipped case become a failure?

## 8. ADRs

_(populated automatically as `/adr-create` links ADRs to this RFC)_
