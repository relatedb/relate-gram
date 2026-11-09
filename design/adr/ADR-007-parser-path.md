---
id: ADR-007
title: Parsing Through the tree-sitter-gram Syntax Tree
status: accepted
rfcs: [RFC-001, RFC-002]
created: 2026-11-09
---

# ADR-007: Parsing Through the tree-sitter-gram Syntax Tree

## 1. Context

The loader has to report the position of every error, reject what does not map to Cypher, and drop nothing silently except documented, reported tags. Those duties need, for every value, key and element, its exact position and its grammatical kind. They also need every malformed construct to be reported as such.

`gram-codec` (crate `relateby-gram`, from the pattern-rs project) is the parser the constitution names. It offers two paths: a hand-written nom parser, and a tree-sitter based one that lowers the syntax tree into pattern-rs's `Pattern<Subject>` values. SPIKE-003 ran both against `tree-sitter-gram` 0.6.1, the grammar this project treats as the reference for gram. The outputs of both paths put properties in a map and drop parts of the source:

- Neither path carries the position of a value. Errors from the nom parser point at the start of the enclosing record.
- A repeated key collapses to its last value.
- An integer too large for 64 bits is a syntax error with a misleading position in one path and a silently missing property in the other.
- A tag written on a fenced string is dropped by both.
- Input the grammar rejects is accepted: an empty array, a property with no value (the CST path counts only ERROR nodes, not the MISSING nodes tree-sitter inserts) and, in the nom parser, a raw newline in a quoted string.
- `010` is read as 10, though the grammar calls it an octal literal.

The syntax tree that `tree-sitter-gram` produces has all of this: a node for every key, value, array element, tag and fenced string, with row, column and byte range, and ERROR and MISSING nodes where the input is malformed. Both `tree-sitter` 0.25 and `tree-sitter-gram` 0.6.1 are published on crates.io. The grammar's own corpus is the conformance reference (Constitution IV).

The constitution's Rust Constraints section requires the crate to depend on `gram-codec` for parsing and says any other parser needs an ADR. This is that ADR.

## 2. Decision

### 2.1 The parser is tree-sitter-gram

The loader parses gram text with the `tree-sitter` crate and the `tree-sitter-gram` grammar crate, pinned to 0.6.1 in `Cargo.toml` and bumped deliberately along with the corpus tests. It reads the resulting syntax tree and builds its own load plan from it. `gram-codec` is not a dependency of the library.

### 2.2 A malformed tree is an error

The loader reads its input as bytes. Input that is not valid UTF-8 is an error at the first bad byte, and a leading UTF-8 byte order mark is skipped. Line endings may be LF or CRLF.

After parsing, if the root node's `has_error` is true, the loader collects every ERROR node and every MISSING node and fails the load. Each is reported as a parse error with its position and, for a MISSING node, the kind of node that was expected. A position is a 1-based line, a 1-based column that counts bytes from the start of the line, and a byte offset from the start of the input. The report lists up to a cap of errors, with the total. A tree without errors goes on to be read; no value is read from a tree that has any.

### 2.3 What the loader reads from the tree

The loader takes these from the tree, each with the position of its node:

- elements: node patterns, relationship patterns (the arrow forms ADR-008 accepts) and paths, each with its identifier, labels and record;
- record properties: the key node (symbol, backtick-quoted or double-quoted name) and the value node, in source order, which keeps repeated keys visible (ADR-006 §2.4);
- values: by node kind, as ADR-006 §2.1 to §2.3 define them, including the `tag` and `content` fields of a tagged string, wherever the string is written, inline or fenced;
- the top-level record (ADR-003).

Escape resolution, integer and decimal conversion and the other value rules of ADR-006 are implemented in the loader on node text. An identifier, label or key keeps the exact text the grammar gives it, with quoting removed.

### 2.4 Constructs outside the subset

These constructs, which RFC-001 excludes, are load errors that name the construct and its position:

- annotations (`annotated_pattern`, with `property_annotation` and `identified_annotation` nodes);
- subject patterns, the bracketed form with elements (`subject_pattern`), and a bare identifier written as an element in one (`pattern_reference`);
- a double colon (`::`) in front of a label or a property key (RFC-001 7.12).

Any other node kind that the loader does not read is also an error that names its kind, so a grammar upgrade that adds a construct fails loudly instead of loading partially.

### 2.5 Comments

The loader ignores every comment: line comments (`//`) and documentation comments (`/** … */`, gramdoc). A comment carries no graph data in a data file, so ignoring it is not a drop under Constitution II, and nothing is reported. A comment inside a malformed region is still covered by 2.2. A later kind of gram file, such as an ontology file, may give comments meaning; that would be decided separately and would not change how data files are read.

### 2.6 Conformance

The corpus of `tree-sitter-gram` is run in tests. It comes from a git submodule of the `tree-sitter-gram` repository at tag v0.6.1, kept in step with the crate version, because the published crate may not ship it. A test also runs the corpus inputs against the crate downloaded from crates.io and checks that its trees equal the submodule's. For each input: every corpus input that the subset accepts must give a load plan that matches what the corpus tree says, and every input the subset rejects must be an error at the node the test expects. The corpus inputs the project excludes are expected errors. The cases the corpus does not settle (for example the value of an octal literal) are decided in the ADR that owns them and tested there.

### 2.7 Constitution amendment

On acceptance, the Rust Constraints section of the constitution changes from "The crate MUST depend on `relate` and `gram-codec` (from pattern-rs) for the engine and the parser; any other parser requires an ADR." to "The crate MUST depend on `relate` for the engine and on `tree-sitter` and `tree-sitter-gram` for parsing (ADR-007); any other parser requires an ADR." The same amendment adds to Principle II: "Comments are not graph data; a loader ignores them, and that is not a drop." This changes a constraint and clarifies a principle, and adds none, so the version becomes 2.1.0. `CLAUDE.md` is updated in the same commit (its Purpose and parser bullets).

## 3. Alternatives Considered

3.1. **Use `gram-codec`'s nom parser.** Rejected: it returns values without positions and loses repeated keys, and it accepts input the grammar rejects.

3.2. **Use `gram-codec`'s tree-sitter path and its `lower` function.** Rejected: it lowers away the information the loader checks (repeated keys, fenced tags, overflowing integers) and reports malformed input as valid.

3.3. **Use `gram-codec` for parsing and add what is missing in the loader's own pre-pass over the source text.** Rejected: the pre-pass would be a second parser for the same grammar.

3.4. **Get pattern-rs to expose positions, repeated keys, tags and a strict error check, and keep `gram-codec`.** Rejected for now: it makes the first loader wait on another project's schedule and API, for properties that the syntax tree already gives. Nothing here prevents adopting `gram-codec` later if it offers them; that would be a new ADR.

3.5. **Use tree-sitter for the loader and keep `gram-codec` as a test-only dependency to cross-check load plans.** Rejected: the two disagree where `gram-codec` is wrong (octal, fenced tags, repeated keys), so a cross-check would flag the loader's correct behavior; the corpus is the reference instead.

3.6. **Write a new parser in Rust.** Rejected: it would duplicate a grammar that is already maintained with its own corpus.

## 4. Consequences

- The loader owns the code that turns syntax-tree nodes into elements and values. It is more code than calling `gram-codec`, and it is where Constitution IV's conformance is tested.
- The crate no longer depends on pattern-rs, and the exporter writes gram text itself under ADR-002 and ADR-006 §2.5; it does not use `gram-codec`'s serializer. Its output is checked by parsing it back through the loader's parser in round-trip tests.
- Bumping `tree-sitter-gram` is a deliberate change that reruns the corpus and may change which inputs load.
- `tree-sitter` is a C library built by a build script, so building the crate needs a C compiler. Whether the loader must also build for WebAssembly (Relate has a WebAssembly build, RFC-001) is not decided; `tree-sitter` can target it, but that is untested here. The Python and TypeScript bindings are not decided (CLAUDE.md, still open) and inherit this.
- The repository gains a git submodule for the corpus.
- The constitution is amended on acceptance (2.7).
- Comments in a data file are lost on load, and an export does not write any. A file that keeps notes in comments loses them on a round trip.
- Open: files whose comments are structured data (an ontology file) need their own decision.

## 5. Related

- RFCs: RFC-001, RFC-002. Resolves RFC-001 7.10 for comments (they are not kept). On RFC-001 7.14, exports do not use `gram-codec`'s serializer; whether they match `pato fmt` stays open.
- SPIKE-003 records the parser behavior this decision rests on.
- ADR-006 builds its value rules on the nodes this ADR reads.
