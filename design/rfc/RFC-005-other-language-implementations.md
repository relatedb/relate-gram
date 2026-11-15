---
id: RFC-005
title: relate-gram in Other Languages
status: review
created: 2026-11-15
adrs: []
---

# RFC-005: relate-gram in Other Languages

## 1. Summary

Relate is an embedded Cypher graph database, used from Rust, Python, Node.js, the browser, and Go, Java and Swift. relate-gram loads a gram file (a text notation for property graphs) into a Relate database and writes a database back out as gram. It exists as a Rust library and a command, and a program in another language can reach it only through the command and a database on disk.

What has to be shared across languages is behavior, not code: which files are accepted, which are rejected and with what error at what position, what graph a file loads to, and what text a database exports to. Behavior can be written down as cases that any implementation runs.

Five ways to serve other languages are compared: wrap the command, bind the Rust library, write native packages, split the work at the statement boundary, and package the tool as a Relate command extension. The preferred direction is to write down the shared cases first and then build native packages, Python first and TypeScript next, each tested against those cases. Seven questions remain open.

## 2. Problem / Motivation

A concrete case: a Python notebook holds an in-memory Relate connection (`relate.connect(":memory:")`) and wants to load `friends.gram` into it. relate-gram cannot do that today.

Relate ships a Python package (`relate-db`, version 1.0.2, built with PyO3), npm packages for Node.js and for the browser (WebAssembly), and clients for Go, Java and Swift. relate-gram is used through its Rust library or through the `relate-gram` command, whose `load` and `export` take a database directory (`--db PATH`). Three costs follow outside Rust:

- A program that already holds a connection cannot hand it to relate-gram. The Rust library takes a connection from the Rust `relate` crate, and the Python package is a separate build of the engine that exposes no handle another library could use. The command opens its own database, and Relate refuses a second writable open of a store that a process already has open, so the program must close its own connection first and cannot use an in-memory database at all.
- The diagnostics (code, line, column, message) are available as data only to Rust callers. Another language has to parse the command's printed output to check, fix or normalize a file.
- The behavior of the tool is specified in detail and tested, and the specification is readable only through Rust. There are 171 fixture and expectation files under `tests/fixtures/` (100 gram files and 71 `.expect` files giving the expected error code, line, column and byte offset), and the expected text of exports is written as golden strings inside Rust test code. A second implementation would have to extract the cases from that code before it could know whether it agrees.

Language bindings were deferred from the first phase of the project, and the property value mapping was written to be a single rule that the loader, the exporter and any later bindings share. This RFC is the design of that later work.

## 3. Goals

3.1. A Python program can run `check`, `load` and `export` on a connection it already holds, in memory or on disk, and passes every case in the shared collection (3.3) that Relate's Python package can express.

3.2. A TypeScript program on Node.js can do the same. The browser is covered to the extent that Relate's WebAssembly package allows (7.2).

3.3. Correct behavior is defined once, in cases that are independent of any implementation language, and every implementation, the Rust one included, runs them.

3.4. A change to behavior (a new rejection, a changed export spelling) starts as a failing case in that collection, and an implementation that disagrees fails.

3.5. An implementation whose Relate package cannot express a case, because a value type or a transaction is missing, skips that case by name, with the reason recorded, and never approximates it. A skipped case is listed in the implementation's documentation.

## 4. Non-Goals

4.1. New gram constructs or new Relate value types.

4.2. Implementations for Go, Java and Swift in the first stage.

4.3. A requirement that every implementation offers every operation. Which operations come first is open (7.7).

4.4. Changing the Rust library's interface or bringing relate-gram into the Relate repository, though 5.3.2 and 5.3.5 note where either would change the options.

4.5. Matching the Rust implementation's speed. Each implementation measures itself.

## 5. Proposed Approach (high-level)

### 5.1 What must be the same

Behavior that a case can check without knowing how an implementation is built:

- The accept or reject decision for a gram text, and for a rejection the diagnostic: code, severity, line, column and byte offset.
- The graph that an accepted text loads into an empty database: nodes, relationships, labels, types and property values with their types.
- The effect of a load on a non-empty database: which elements are matched by identifier and which are created.
- Atomicity: a load that fails changes nothing, and a load started inside an open transaction is refused.
- The load's settings (the limit on reported errors, the choice of identity property) and its report (counts of what was created and updated, and warnings).
- The text a database exports to, byte for byte, including identifiers, quoting and the canonical spelling of values.
- The text `fix` and `normalize` produce, for the implementations that offer them.

Parsing, diagnostics and value spelling need no database. Loading and exporting reach the database only through Cypher statements with parameters, and through transactions.

### 5.2 A shared definition of correct behavior

The cases exist as Rust fixtures and expectations, with the expected exports embedded in test code. They can be collected in a form that is not tied to Rust: gram files with the expected outcome (the loaded graph, or the diagnostics with positions), databases described by their contents with the expected exported text, and the table of diagnostic codes. Every implementation reads the same collection and reports the cases it fails. The Rust library's own tests would read it too, so the collection has one owner and Rust is one consumer among several.

`gram-value` runs its languages this way: one corpus of cases, a list of skipped cases per package with reasons, and a table of results per language generated in continuous integration. The same mechanism answers how skipped cases are recorded (3.5). Extracting the cases is also the cheapest step in this RFC: it needs no code in another language and it removes the golden strings from the Rust test code.

### 5.3 Candidate strategies

The strategies are compared on the same axes in the table that follows 5.3.5.

**5.3.1 Wrap the command.** A package for each language runs `relate-gram` as a subprocess and parses its output.

**5.3.2 Bind the Rust library.** Compile the library into a Python extension module and a Node.js addon. The library works on a connection from the Rust `relate` crate, so the binding can do one of three things:
- open the database a second time, which Relate's single-writer lease refuses unless the host's own connection is closed;
- change the library to work through a small connection interface that the host fills in with callbacks, which changes the library's interface and puts a value conversion for every Relate type on the language boundary;
- ship relate-gram inside Relate's own Python wheel and npm package so it shares the engine, which requires it to live in the Relate repository (the project allows that later, and 4.4 does not rule it out).

A fourth route is a connection interface implemented over Relate's C interface, which the Node, Go, Java and Swift clients already use. A library written against such an interface would serve those languages at once.

**5.3.3 Native packages.** A package for each language written in that language, built on the language's `gram-value` package, a tree-sitter-gram binding, and the language's Relate package, sharing only the cases of 5.2 with the Rust library. The logic is written again for each language, about 5,000 lines in Rust today. The port is smaller than that figure suggests: the known-tag content rules now come from `gram-value`, which every language has.

**5.3.4 A Relate command extension.** Relate's command line can run a child process that emits batches of parameterized statements; the host owns the database and executes them (Relate's extension decisions, ADR-0429 and ADR-0447). That fits `load`. It cannot offer `export` or the identity scan, which read the store, because the child has no access to it. It serves people who use the Relate command and not programs holding a connection.

**5.3.5 Split at the statement boundary.** One implementation does the parsing, diagnostics and planning, and emits the Cypher batches as data; a thin executor in each language runs them on the connection the program holds. The extension protocol already works this way for loading. It keeps one parser and one planner and needs no connection hand-off. The obstacles are that the scan and resolve step of a load reads the database, and so does export, so each needs a round trip between the planner and the executor, and the planner still has to run somewhere that every language can reach.

| | Reuses the program's connection | In-memory database | Atomic load | Browser | Logic written per language | Native build burden |
|---|---|---|---|---|---|---|
| 5.3.1 wrap the command | no | no | yes, inside the tool | no | none | a binary per platform |
| 5.3.2 bind the library | only with callbacks or inside Relate's package | only the same ways | if the host exposes transactions | no | none (callbacks: a conversion layer) | a native module per platform |
| 5.3.3 native packages | yes | yes | where the Relate package has transactions (Python and Node: yes; browser package: no) | TypeScript, where a parser reaches WebAssembly | all of it | none of its own; the tree-sitter and Relate packages are compiled |
| 5.3.4 extension | not applicable | not applicable | the host's | no | none | a build of the extension |
| 5.3.5 split | yes | yes | needs a transaction in the executor | possible | the executor only | a native planner reachable from every language |

### 5.4 Preferred direction

Write the shared cases first (5.2), then build native packages (5.3.3), Python first and TypeScript next, each tested against the cases from its first commit. Python is first because its Relate package is mature (1.0.2), exposes transactions and has a vector type, which is the hardest value type to carry. TypeScript follows and starts with Node.js. The split at the statement boundary (5.3.5) is the strongest alternative, because it keeps one parser and planner; a short spike should show whether its round trips are manageable before the native packages are committed to (7.8). Wrapping the command (5.3.1) remains available to a program that accepts its limits.

### 5.5 Cross-cutting concerns

- **Value types.** Each Relate package represents values differently, and an implementation can only store or return what its package can. A case it cannot express is skipped with a reason (3.5).
- **Versions.** The implementations and the Rust library describe the same behavior; whether they share version numbers is open (7.6).
- **Generated code.** Generating the diagnostics table, the tag profile and the case reader from one description is likely to help in each language. It is a tool choice for the decisions that follow.
- **Distribution.** A native package contains no compiled code of its own, but it depends on the tree-sitter and tree-sitter-gram bindings and on Relate's package, which are compiled for each platform. Whether prebuilt binaries exist for each platform an implementation targets is open (7.3).

## 6. Alternatives Considered

6.1. **Stay Rust-only and document the command.** Rejected: the costs of section 2 remain for every program outside Rust.

6.2. **One Rust core compiled to WebAssembly and loaded from every language.** Rejected: the core needs a database connection on the other side of the boundary, so the connection problem of 5.3.2 returns. A parser also has to be built for WebAssembly, which tree-sitter-gram has no browser binding for today; every strategy that reaches the browser needs that build, so it does not tell the options apart.

## 7. Open Questions

7.1. What happens to `F16` and `BF16` embeddings in an implementation that cannot set the process environment before connecting? Relate refuses 16-bit vectors unless `RELATE_PREVIEW=float16-coordinate-types` is set, which a Python or Node.js program can do before it connects and a browser page cannot.

7.2. Which statement forms can the browser package run? It accepts parameters only as JSON, builds typed values in Cypher, and has no explicit transactions (each statement commits). Can a load be atomic there, or is the browser outside the goals, and how does the parser reach the page (a WebAssembly build of tree-sitter-gram)?

7.3. Do tree-sitter and tree-sitter-gram publish prebuilt binaries for every platform the implementations target, for Python and Node.js?

7.4. Where does the shared collection of cases live: in this repository, in a repository of its own, or alongside the `gram-value` corpus? It needs one owner and must be readable from every language's test suite.

7.5. In what form is a case written, and how is a loaded graph or a database's contents compared? The Rust tests compare database dumps and golden strings, and the form must express both.

7.6. Do the implementations share version numbers with the Rust library, or version independently against the behavior they pass?

7.7. Which operations does each implementation offer first: `check`, `load`, `export`, `normalize`, `fix`, a command-line tool?

7.8. What would a spike of the split at the statement boundary (5.3.5) have to show: the number of round trips for a scan and an export, and whether the planner can run where every language can call it?

## 8. ADRs

_(populated automatically as `/adr-create` links ADRs to this RFC)_
