---
id: SPIKE-003
title: Parser Behaviors of gram-codec and tree-sitter-gram
status: done
rfcs: [RFC-002, RFC-001]
created: 2026-11-09
---

# SPIKE-003: Parser Behaviors of gram-codec and tree-sitter-gram

## 1. Question

Can the loader take its parse results from `gram-codec` (crate `relateby-gram` 0.7.2, pattern-rs), or does it need the grammar's syntax tree itself? The loader must report the position of every error (Constitution VI) and must not drop anything silently (Constitution II). So the parser has to deliver exact positions, report malformed input, and keep what ADR-001 and ADR-006 check: duplicate keys, tags, array elements, integer overflow and empty keys.

## 2. Method

A throwaway Rust binary, `scripts/probe`, parses about forty small gram inputs with three paths: `gram_codec::parse_gram` (a nom parser), `gram_codec::parse_gram_cst` followed by `lower` (a tree-sitter based path), and `tree-sitter-gram` 0.6.1 directly. It prints each path's result. Run with `cargo run --manifest-path design/spikes/SPIKE-003-parser-behaviors/scripts/probe/Cargo.toml`, which needs pattern-rs checked out at `/Users/abk/Developer/relateby/pattern-rs` with its `external/tree-sitter-gram` submodule at v0.6.1. Output: `probe-output.txt`. The corpus pass rate comes from pattern-rs's own `corpus_integration` test.

## 3. Time-box

Two hours.

## 4. Findings

- **Corpus:** pattern-rs's conformance test passes 144 of 201 tree-sitter-gram corpus cases, with 39 more skipped as expected errors. The 18 failures are annotation syntax (out of scope here), gramdoc comments, and one array-in-map case that is a gap in the test's S-expression comparison; the parsed value is right.
- **Scalar arrays in maps:** accepted by every path.
- **Positions:** `parse_gram` returns patterns with no positions. Its syntax errors carry line, column and offset, but point at the start of the enclosing record, not at the bad value. The CST path gives element-level byte spans only. Raw tree-sitter gives the row, column and byte range of every node.
- **Duplicate keys:** `{k: 1, k: 2}` becomes `k: 2` in both `gram-codec` paths. tree-sitter shows two `record_property` nodes.
- **Integer overflow:** `9223372036854775808` makes `parse_gram` fail (at the record start); the CST path lowers it to an empty record with no error. tree-sitter shows a plain `integer` node.
- **Tags:** an inline tag (``md`text` ``) is kept by `parse_gram`. A fenced tag (a tag after the opening fence) is dropped by both paths, which return a plain string. tree-sitter shows a `tagged_string` node with `tag` and `content` fields.
- **Malformed input accepted:** `(a {xs: []})` and `(a {k: })` are errors in tree-sitter (a MISSING node under the array and the property). The CST path reports them as valid because only ERROR nodes are counted, and lowers them to `{xs: []}` and `{}`. `parse_gram` accepts `[]` and rejects `{k: }`. `parse_gram` also accepts a raw newline inside a single-, double- or backtick-quoted string, which the grammar rejects.
- **Control characters:** a raw tab, CR, 0x01 and DEL are accepted inside a quoted string by the grammar. A raw NUL cannot be parsed. A fenced string's `string_content` starts after the newline that follows the opening fence and keeps its final newline (`"body\n"`) and any backslashes (`a\nb` stays as typed).
- **Octal:** `010` is an `octal` node in the grammar. `gram-codec` reads it as the decimal 10. The corpus pins only the node kind (`042` is `(octal)`), not the value. `08` is an error.
- **Decimals:** `1e5` is an error. A decimal with several hundred digits parses as infinity in `parse_gram` and as a valid `decimal` node in tree-sitter.
- **Keys:** an empty quoted key is accepted by every path (the tree-sitter node is a `double_quoted_name` with empty content). A bare `true` is an ordinary key. `@id` as a key is a syntax error everywhere.
- **Comments inside quoted text (found while implementing the exporter, 2026-11-09):** in `tree-sitter-gram` 0.6.1 a quoted string or name whose text starts, after any blanks, with `//` fails to parse (`(a {k: "//x"})`, `(a {k: " //x"})`, ``(a:`//CB`)``). A text that starts with `/**` fails too when a `*/` appears anywhere later in the file, because the documentation comment then matches more than the text does. The same text with a leading escape (`"\/\/x"`, `"\/**x"`) parses, and a `//` that is not at the start of the text (`"http://x"`, `"a//b"`) is fine. This looks like a grammar defect (the comment tokens compete at the start of a quoted text); it is worth reporting to tree-sitter-gram. Strings and names read from files that start with `//` cannot be loaded, and the exporter writes such a leading slash as `\/`.
- **Availability:** `tree-sitter-gram` 0.6.1 and `relateby-gram` 0.7.2 are both on crates.io.

## 5. Conclusion

`gram-codec` loses or hides several things the loader's rules depend on (positions, duplicate keys, overflowing integers, fenced tags) and accepts input the grammar rejects. A loader that reads the tree-sitter-gram syntax tree itself has all of these, and ADR-007 decides that it does. ADR-006's rules for escapes, fenced content and arrays match the tree; two statements need correcting (raw control characters and NUL, octal), recorded in ADR-006.
