---
id: ADR-013
title: gram-value Is the Value Model for Load and Export
status: proposed
rfcs: [RFC-002]
created: 2026-11-15
---

# ADR-013: gram-value Is the Value Model for Load and Export

## 1. Context

relate-gram holds a gram property value in memory in its own type, `PropValue` (`src/values.rs`), an enum of an integer, a float, a boolean, a string, a validated typed value and a list. On export it reads values back from the database as Relate values and writes each with its own functions (`scalar_text`, `float_text`, `names::string_literal`, `temporal_value_text` in `src/export.rs`). These are a second model of what `gram-value` already defines: eleven value kinds, equality, a total order and one canonical text, with packages for Rust, TypeScript, Python, Go and Swift checked against a shared conformance corpus. ADR-012 already moved the validation of the eight typed tags to `gram-value`, so the crate is a dependency and the two models now sit side by side in `typed/` and `values.rs`.

The two models agree on most of what they write. Integers, booleans, shortest-decimal floats and the string escapes (`\" \\ \n \r \t \b \f`) are the same. They differ in one visible place: relate-gram always double-quotes a string (`status: "active"`), and the canonical text writes the bare word when the text is a symbol (`status: active`). A second difference is what the loader can say about a value. `PropValue` has no equality beyond Rust's derive, no canonical text and no relation to the values a TypeScript or Python binding will hold; every binding would have to describe `PropValue` or map to something else.

Three facts about the code shape the decision:

- The loader's value carries more than a value. Typed values carry a source span (`TypedValue.span`) so a database rejection can be reported at the tagged string, and arrays are checked element by element with the position of the first element that differs. `gram_value::Value` has no positions.
- The lowering from tree-sitter nodes to values (`convert_value`, `convert_record`) is relate-gram's code, because `gram-value` does not parse. It enforces the strictness rule: ranges, measurements, nested maps, octal literals and integers outside 64 bits are rejected with a located error.
- The exporter's property order is already `gram-value`'s: a record is a sorted map by key, ascending in bytes. Keys are written differently (relate-gram backticks a key that is not a symbol; `gram-value` double-quotes), and elements, identifiers and labels are not values at all.

RFC-002 asks for one documented rule shared by the loader, the exporter and later bindings (its goal 3.5). ADR-006 2.5 answered the canonical spelling of a written value (question 7.2) with relate-gram's own rules; this ADR changes that answer for strings by adopting the shared model.

## 2. Decision

### 2.1 The loader holds `gram_value::Value`

`PropValue` is removed. Lowering produces a `gram_value::Value` of kind `Integer`, `Decimal`, `Boolean`, `String`, `Tagged` (a recognized tag) or `Array` of one of these kinds, together with the position of the value and, for an array, the position of each element. Those two positions are all that later steps use: a database rejection of a typed value is reported at the tagged string or at the array element, and `value.array-mixed` is reported at the element during lowering. No later step uses the position of a nested part beyond them, so no per-node tree of positions is kept.

A record stays an ordered list of key and value, as today: the order of writing is kept (the message for a record that is not stored lists keys in it), and duplicate keys are detected during lowering with the key's position. `gram_value::Map` is not used for a record, since it sorts its keys and `Value::map` reports a repeated key without a position. A nested map in a value is still rejected by the lowering.

### 2.2 The lowering keeps the strictness rule

`convert_value` keeps its structure and its diagnostic codes; it builds `Value`s instead of `PropValue`s:

| Gram value | Result |
|---|---|
| decimal integer | `-0` is read as the integer 0 (the grammar accepts it and the loader stores 0 today; `integer_from_digits` rejects it). Any other text goes to `Value::integer_from_digits`; a `BigInteger` result is `value.integer-range` with the existing message. An error from it cannot occur for text the grammar's integer token matched |
| hexadecimal integer | parsed to `i64` locally; out of range is `value.integer-range` |
| decimal | `Value::decimal_from_text`. A `BigDecimal` result means the digits are not a float that writes back the same, which covers precision loss, overflow and underflow; the loader parses its digits as a 64-bit float and applies the existing checks: non-finite, or a non-zero literal that rounds to zero, is `value.decimal`; any other result is stored as that float with no loss reported (3.5) |
| boolean, symbol, quoted string | `Value::boolean`, `Value::string` |
| tagged string, recognized tag | `Tag::validate` (ADR-012), then `Value::tagged(tag, normal_content)` |
| tagged string, other tag | a `Value::string` with the drop warning or the `--tags error` error (ADR-006 2.3) |
| array | the elements lowered; one kind required (below) |
| range, measurement, map, octal | rejected with the existing codes |

The homogeneity rule of arrays stays the loader's own: elements must have one `Value::kind()` and, for tagged elements, one tag. (`Value::is_homogeneous_array` compares kinds only, so it would let a `date` array hold a `datetime`.) The first element that differs gets `value.array-mixed` at its position, and the message names the tag for a tagged element (`date`, not `tagged`), as it does today.

A decimal literal that a 64-bit float cannot hold exactly (a `BigDecimal`) loads silently as the nearest float, as it does now. Reporting that loss is not part of this decision (4).

### 2.3 Typed values carry the normal content

After validation a typed value is `Value::tagged(tag, normal_content)`, where the content is gram-value's normal content string. `TypedValue`, `TypedContent` and `PointContent` are removed. `Tag::validate` returns that string. The write step of the loader, which builds typed values through Cypher constructors (ADR-011 2.7), reads the tag and content from the value: text tags pass the content to the constructor, and a point calls `gram_value::tags::interpret_tag("point", content)` again to obtain its SRID and numbers. The second interpretation of a point is cheap and keeps one place that knows a point's text form. A point's content now also takes part in matching a database message to a value (`typed_failure`), which today returns no text for a point; that is intended, since it can only make a rejection more precise.

### 2.4 Conversion to Relate

One conversion from `Value` to a Relate value exists, for `Integer`, `Decimal`, `Boolean`, `String` and `Array` (to a list). A `Tagged` value takes the typed write path of 2.3, and the other kinds are excluded by the lowering. The conversion returns an error for a kind it does not map, so the exclusion is checked, not assumed.

### 2.5 The exporter writes each property value as canonical text

On export, each property value read from the database becomes a `gram_value::Value` and is written with `Value::to_text()`:

| Relate value | `gram_value::Value` |
|---|---|
| integer, boolean, string | `integer`, `boolean`, `string` |
| float | `decimal` (a non-finite float is the existing export error) |
| list | `array` of the elements; an empty list is `[]` (ADR-011 2.5). A nested list and a list of mixed kinds stay export errors (ADR-006 2.6); the exporter checks these before building the `Value`, because it must name the element and key |
| UUID | `tagged("uuid", lower-case 8-4-4-4-12)` |
| point | `tagged("point", "SRID=n;x y [z]")` with the coordinates of ADR-012 2.6 |
| date, time, datetime, localtime, localdatetime, duration | `tagged(tag, Relate's toString)`, as today |
| anything else | an export error as in ADR-006 2.6 |

Elements, identifiers, labels, relationship types and record keys are not values and are written as now (`names::identifier`, `names::name`). A string containing U+0000 stays an `export.value` error, checked by the exporter before the value is built, for a string property and for a string in a list (ADR-006 2.6, ADR-011 2.6): `gram-value` writes it raw and the grammar cannot read it back. The writer's `unwritable` diagnostics cannot occur for the values the exporter builds, since nested lists are rejected earlier; they are mapped to `export.value` errors with the element and key as a fallback. `names::string_literal` and the exporter's `scalar_text`, `float_text` and `temporal_value_text` are removed when nothing else uses them.

### 2.6 What changes in the written file

- A string that is a symbol is written bare: `status: active`, `name: Ada`; `"Ada Lovelace"`, `"true"` and the empty string stay quoted. This amends ADR-006 2.5, whose table says a string is double-quoted.
- Every other value is written as before, including points (ADR-012) and the other typed values.
- A string value no longer carries the escape of `/` in its leading run (ADR-006 2.5, from SPIKE-003). It was written because tree-sitter-gram 0.6.1 read a quoted text starting with `//`, or with `/**` when a `*/` follows, as a comment; 0.6.2 and the pinned 0.7.0 read it as a string, and ADR-006 kept the escape so that exports also parse with 0.6.1. The cost is that a string value an export writes with such a start cannot be read by a tool that still uses tree-sitter-gram below 0.6.2. Backtick-quoted names (identifiers, labels, relationship types and keys) keep the escape, because `names::escape_into` serves them and this ADR does not change them (2.5). Before the string escape is removed, a deterministic round-trip test writes strings that begin with `//`, with `/**` followed later by `*/`, and with leading blanks, and loads them with tree-sitter-gram 0.7.0; if any fails to read back equal, the escape stays.

### 2.7 What stays

`Syntax`, `Span`, the diagnostics and their codes, the tag policy, identity and `@id` handling, the record layout of the exporter and the load plan keep their shapes; the plan's types and `plan::value_text` change only where they name `PropValue`. The fixer and the renderer do not name it and are unaffected.

## 3. Alternatives Considered

3.1. **Keep `PropValue` and call gram-value's writer only for export.** Rejected: the loader would keep a second model of every value, and each binding would still have to describe `PropValue` as well as `Value`. It saves the plumbing of 2.1 and leaves the divergence.

3.2. **A recursive tree of values with a position on every node.** Rejected: the only later uses of positions are a typed value's position and an array element's position (a database rejection of a tagged string, and `value.array-mixed`), which the value's own position and the element positions cover. A full tree repeats the structure of `Value`.

3.3. **Use `gram_value::Map` for a record.** Rejected: it sorts its keys, while the loader keeps the order of writing, and `Value::map` reports a repeated key without a position, so it cannot replace the loader's duplicate check.

3.4. **Write the canonical text but quote every string.** Rejected: it is a local rule on top of the shared writer, and the point of the shared text is that no tool carries a different one.

3.5. **Report a precision-losing decimal as an error or a warning now.** Not decided here: ADR-006 accepted such a literal and rounding it silently; the `precision` loss kind of `gram-value` is the place to report it, and the change affects released behavior. It is a separate decision (4).

3.6. **Adopt `Value` for the exporter only now and the loader later.** Rejected: the exporter's mapping from Relate values is the smaller half, but the loader's value type is what the bindings will meet first, and a release with two models gains little over one.

## 4. Consequences

- Library API changes against 0.1.0, so the next release is a breaking minor version for library users (0.x): `values::PropValue` is removed, `plan::PlannedNode.props`, `PlannedRel.props` and `plan::value_text` take other types, `names::string_literal` is removed, and `Tag::validate` and the typed content types (unreleased, listed in the changelog's Unreleased section) change shape. The command line is unchanged.
- Exported files change for every string property that is a symbol (bare instead of quoted); a user who diffs exports sees one change. Typed values and points are written as before. Releasing this together with typed tags (0.2.0) makes it the only export change users see. Strings, keys and identifiers that begin with `/` are covered by 2.6.
- Code shrinks where the exporter's value writers go and grows slightly where positions are threaded through `plan.rs` and `load.rs`. The 122 contract tests and the round-trip and position tests are the safety net. These change: the golden export strings in `tests/contract/export_basic.rs`, `export_values.rs` and `export_identity.rs` and in `tests/cli.rs` (`strs: [a, b]`, `email: x@y`), the unit tests in `src/typed/mod.rs` that match the typed content types, and the `string_literal` tests in `src/names.rs`.
- A point is interpreted twice (validation, then the write step). The performance test is re-run against the figure measured after ADR-012 (typed load of 10,000 elements, 348.7 ms in `specs/003-gram-value-tags/tasks.md`); a result more than 1.2 times that sends 2.3 back for reconsideration.
- The outer syntax of every property value (quoting, escapes, arrays, tag fences) now follows `gram-value`'s canonical text. The content of a tagged value is still what Relate prints (`toString`) or what ADR-012 writes for a point; it is not run through `interpret_tag`, so it need not equal the normal content. A change of the canonical text in a later minor version of `gram-value` needs a deliberate bump and a look at the golden tests, as with the tags in ADR-012.
- The string escape of a leading `/` is removed on the condition in 2.6; if tree-sitter-gram 0.7.0 does not read such strings back, the escape stays and the fix belongs in the grammar.
- Follow-up work: report a precision-losing decimal through gram-value's loss kinds (3.5); update the documents that name `PropValue` or show exported strings: `specs/001-phase1-load-export/data-model.md` and `contracts/library-api.md`, `specs/002-typed-tags/` (research, data-model, plan), `specs/003-gram-value-tags/spec.md`, and the changelog; decide the bindings' value type on top of this one (open in `CLAUDE.md`); a Spec Kit specification and tasks (`specs/004-gram-value-model/`).

## 5. Related

- RFC-002 (property value mapping): goal 3.5 (one rule shared by loader, exporter and later bindings) and question 7.2 (canonical spelling).
- ADR-006 (property value mapping; its 2.5 string spelling and 2.6 export errors are amended here), ADR-011 (typed tags; 2.7 typed write path), ADR-012 (known tags from gram-value, the first step of this adoption), ADR-007 (the parser path that supplies the nodes).
- `gram-value`: https://github.com/gram-data/gram-value (its ADR-002 the kinds, ADR-003 the canonical text, ADR-005 the result shape).
