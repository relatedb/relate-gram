# gram-value: a shared home for gram's data values

Gram is a text notation for graphs and patterns, and every gram file puts values on its elements: strings, numbers, booleans, arrays, maps, ranges, measurements and tagged strings such as ``date`2024-01-15` ``. This note records an idea that came out of building `relate-gram`, the crate in this repository that loads gram into the Relate graph database and exports it back: the code that models and checks those values is not specific to Relate, and could live in its own project, `gram-value`, that `relate-gram` and other gram tools depend on. It binds no decision. The foundational RFC for `gram-value` will be written in that project's own repository (created by the project owner at `gram-data/gram-value`, empty at the time of writing), and the choices below become that RFC's questions and its ADRs.

## Why

- **Most of this crate does not need Relate.** Only `load.rs`, `export.rs`, `lib.rs` and `main.rs` use the `relate` crate. The value model (`values.rs`), the eight typed tags with their content forms (`typed/`, ADR-011), the diagnostics, the tree-sitter parse, the strict rules, and the fixer have no database dependency, about 3,000 of 4,300 lines.
- **Gram tools keep rebuilding the value model.** The pattern-rs project (`/Users/abk/Developer/relateby/pattern-rs`) has two: a `Value` with ten variants in `pattern-core/src/subject.rs`, and another in `gram-codec/src/value.rs`. ADR-007 explains why this crate could not use them: SPIKE-003 found that they lose key order and repeated keys (a `HashMap`), the spelling of a measurement (`5kg` and `5.0kg` are one number), and the difference between the integer spellings.
- **Language bindings want the smallest unit.** A value model with no parser and no database can be built for WebAssembly and wrapped for JavaScript and Python without bringing either along (SPIKE-004 measured the parser for WebAssembly separately).

## The idea

`gram-value` models every value that gram can write: integers, decimals, booleans, strings, symbols, ranges, measurements, arrays, maps, and tagged strings with any tag, which stay a tag and its content. It does not model any one consumer's subset. A consumer such as `relate-gram` (which accepts only values that become a Cypher type) or pattern-rs (which accepts all of them) decides for itself which values it supports, and `gram-value` does not name those subsets or ship a profile for them. What it gives every consumer is the means to ask what a value is and to write it back as gram text.

Three further commitments, from the project owner:

- **Subsets are the consumer's business.** The crate does not enumerate "the Relate subset" or "the pattern-rs subset"; a consumer that supports fewer types rejects the others with its own errors.
- **Known tags are an interpretation, not part of the value.** A tagged string stays a tag and a content string. A registry of known tags, such as the eight of ADR-011 and their ISO 8601, extended well-known text and UUID forms, interprets them, and the unknown tags that other projects use for their own purposes (pattern-rs uses `ts`, `re`, `zod`, `cypher`, `pydantic`, `gram` to name constraint dialects) pass through untouched.
- **Standard mappings to host languages, even lossy.** `gram-value` defines how each value maps to a native value in Rust, JavaScript and Python, in a documented table per language. Where a mapping loses information (a 64-bit integer in a JavaScript number, a measurement in a language without one, a range) the table says what is lost, and a consumer can tell that a conversion lost something.

## What this repository would contribute

- The typed-tag content forms and their ISO 8601 relation (ADR-011 and the data model of `specs/002-typed-tags/`), with the Relate-only restrictions marked: the four SRIDs 4326, 4979, 7203 and 9157, and the choice to exclude vectors, belong to `relate-gram`, not to a general value model.
- The fidelity evidence: SPIKE-003 (what existing parsers lose), SPIKE-005 (how each type round-trips through text).
- A parse layer that keeps exact positions and spellings in the syntax tree, which lets `gram-value` stay semantic. The value crate does not need to remember how a value was written, only what it is.

## Where it sits

```text
gram-value      all gram values; known-tag interpretation; text writing; host-language mappings
   ^       ^
   |       +-- pattern-rs (replaces its two Value types; a breaking change there)
gram-graph      the property-graph kind: graph model, parse with positions, the strict rules (possibly this repository's core)
   ^
relate-gram     gram <-> Relate database, gram-graph <-> Relate result graph, the Relate command
```

`gram-graph` is not decided: it could stay inside this repository as a workspace crate until its interface settles, then move. RFC-005 (a draft on the unpushed branch `rfc-005-graph-model`) proposes a Relate-independent graph model and would narrow to that layer.

## Questions for the gram-value RFC

- Does a value record only what it means, or also how it was written (hex or decimal, quoting style)? The leaning here: only what it means, with spellings and positions kept by the parser.
- How is a map stored: ordered, and do duplicate keys survive or fail?
- What is an octal literal worth (`010` is 8 or 10: ADR-006 rejects it, pattern-rs reads 10), and how wide is an integer?
- Is a decimal a 64-bit float, or an exact decimal? Is a measurement a number and a unit, with the spelling kept?
- How does a consumer extend the known-tag registry, and how does it say it supports only some tags?
- Where do the text-writing rules live (string quoting, decimal spelling, name quoting): `gram-value` or a sibling crate?
- How are the host-language mappings specified and tested, and how is loss reported?
- How does the enum evolve (`#[non_exhaustive]`) when three projects depend on it?
- Where do `gram-value` and `gram-graph` live (the `gram-data` organization seems natural), and how does pattern-rs adopt them?

## Meanwhile, in this repository

Nothing structural changes until `gram-value` has a repository and a foundational RFC. Two habits make the later move cheap: keep `src/typed/` free of tree-sitter and `relate` (it is today), and note, when adding value logic here, whether it is generic or Relate-only. The adoption of `gram-value` would be an ADR in this repository, with the constitution's Rust constraint amended as ADR-007 amended the parser constraint.
