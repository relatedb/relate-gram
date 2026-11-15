---
id: ADR-013
title: gram-value Is the Value Model for Load and Export
status: proposed
rfcs: [RFC-002]
created: 2026-11-15
---

# ADR-013: gram-value Is the Value Model for Load and Export

## 1. Context

relate-gram holds a gram property value in memory in its own type, `PropValue` (`src/values.rs`), an enum of an integer, a float, a boolean, a string, a validated typed value and a list. On export it reads values back from the database as Relate values and writes each with its own functions (`scalar_text`, `float_text`, `names::string_literal`, `temporal_value_text` in `src/export.rs`). These are a second model of what `gram-value` already defines: eleven value kinds, equality, a total order and one canonical text, published for Rust, TypeScript, Python, Go and Swift and checked against a shared conformance corpus. ADR-012 already moved the validation of the eight typed tags to `gram-value`, so the crate is a dependency and the two models now sit side by side in `typed/` and `values.rs`.

The two models agree on most of what they write. Integers, booleans, shortest-decimal floats and the string escapes (`\" \\ \n \r \t \b \f`) are the same. They differ in one visible place: relate-gram always double-quotes a string (`status: "active"`), and the canonical text writes the bare word when the text is a symbol (`status: active`). A second difference is what the loader can say about a value. `PropValue` has no equality beyond Rust's derive, no canonical text and no relation to the values a TypeScript or Python binding will hold; every binding would have to describe `PropValue` or map to something else.

Three facts about the code shape the decision:

- The loader's value carries more than a value. Typed values carry a source span (`TypedValue.span`) so a database rejection can be reported at the tagged string, and arrays are checked element by element with the position of the first element that differs. `gram_value::Value` has no positions.
- The lowering from tree-sitter nodes to values (`convert_value`, `convert_record`) is relate-gram's code, because `gram-value` does not parse. It enforces the strictness rule: ranges, measurements, nested maps, octal literals and integers outside 64 bits are rejected with a located error.
- The exporter's property order is already `gram-value`'s: a record is a sorted map by key, ascending in bytes. Keys are written differently (relate-gram backticks a key that is not a symbol; `gram-value` double-quotes), and elements, identifiers and labels are not values at all.

RFC-002 asks for one documented rule shared by the loader, the exporter and later bindings (its goal 3.5), and leaves the canonical spelling of a written value open (question 7.2). This ADR answers that question for property values by adopting the shared model.

## 2. Decision

### 2.1 The loader holds `gram_value::Value`

`PropValue` is removed. Lowering produces a `Located`, a small struct that pairs a value with where it was written:

- `value: gram_value::Value`, one of `Integer`, `Decimal`, `Boolean`, `String`, `Tagged` (a recognized tag) or `Array` of one of these kinds;
- `span: Span`, the position of the value's node;
- `items: Vec<Span>`, the positions of an array's elements in order (empty for a scalar).

A record is `Vec<(String, Located)>`, as the record is today (`Vec<(String, PropValue)>`): the order of writing is kept, and duplicate keys are found with the key's position. `gram_value::Map` is not used for a record, since it is sorted and has no positions. A nested map in a value is still rejected by the lowering.

### 2.2 The lowering keeps the strictness rule

`convert_value` keeps its structure and its diagnostic codes; it builds `Value`s instead of `PropValue`s:

| Gram value | Result |
|---|---|
| decimal integer | `Value::integer_from_digits`; a `BigInteger` result is `value.integer-range`, as now |
| hexadecimal integer | parsed to `i64` locally; out of range is `value.integer-range` |
| decimal | `Value::decimal_from_text`; a `BigDecimal` result is loaded as the nearest finite float, as the loader does today; non-finite and "rounds to zero" stay `value.decimal` |
| boolean, symbol, quoted string | `Value::boolean`, `Value::string` |
| tagged string, recognized tag | `Tag::validate` (ADR-012), then `Value::tagged(tag, normal_content)` |
| tagged string, other tag | a `Value::string` with the drop warning or the `--tags error` error (ADR-006 2.3) |
| array | the elements lowered; one kind required (below) |
| range, measurement, map, octal | rejected with the existing codes |

The homogeneity rule of arrays stays the loader's own: elements must have one `Value::kind()` and, for tagged elements, one tag. (`Value::is_homogeneous_array` compares kinds only, so it would let a `date` array hold a `datetime`.) The first element that differs gets `value.array-mixed` at its span from `items`.

A decimal literal that a 64-bit float cannot hold exactly (a `BigDecimal`) loads silently as the nearest float, as it does now. Reporting that loss is not part of this decision (4).

### 2.3 Typed values carry the normal content

After validation a typed value is `Value::tagged(tag, normal_content)`, where the content is gram-value's normal content string. `TypedValue`, `TypedContent` and `PointContent` are removed. `Tag::validate` returns that string. The write step of the loader, which builds typed values through Cypher constructors (ADR-011 2.7), reads the tag and content from the value: text tags pass the content to the constructor, and a point calls `gram_value::tags::interpret_tag("point", content)` again to obtain its SRID and numbers. The second interpretation of a point is cheap and keeps one place that knows a point's text form.

### 2.4 Conversion to Relate

One function, `to_relate(&Value) -> relate::Value`, maps `Integer`, `Decimal`, `Boolean`, `String` and `Array` (to a list). The other kinds cannot reach it because the lowering excludes them; a `Tagged` value takes the typed write path of 2.3.

### 2.5 The exporter writes each property value as canonical text

On export, each property value read from the database becomes a `gram_value::Value` and is written with `Value::to_text()`:

| Relate value | `gram_value::Value` |
|---|---|
| integer, boolean, string | `integer`, `boolean`, `string` |
| float | `decimal` (a non-finite float is the existing export error) |
| list of one kind | `array`; a nested list, an empty-kind or mixed list stays an export error as in ADR-006 2.6 |
| UUID | `tagged("uuid", lower-case 8-4-4-4-12)` |
| point | `tagged("point", "SRID=n;x y [z]")` with the coordinates of ADR-012 2.6 |
| date, time, datetime, localtime, localdatetime, duration | `tagged(tag, Relate's toString)`, as today |
| anything else | an export error as in ADR-006 2.6 |

Elements, identifiers, labels, relationship types and record keys are not values and are written as now (`names::identifier`, `names::name`). The writer's `unwritable` diagnostics are mapped to `export.value` errors with the element and key, as the other value errors are. `names::string_literal` and the exporter's `scalar_text`, `float_text` and `temporal_value_text` are removed when nothing else uses them.

### 2.6 What changes in the written file

- A string that is a symbol is written bare: `status: active`, `name: Ada`; `"Ada Lovelace"`, `"true"` and the empty string stay quoted. This amends ADR-006 2.5, whose table says a string is double-quoted.
- Every other value is written as before, including points (ADR-012) and the other typed values.
- The special escape of `/` in the leading run of a string (ADR-006 2.5, from SPIKE-003, written for tree-sitter-gram 0.6.1) disappears with `string_literal`. Before it is removed, a test writes strings that begin with `//` and `/**`, exports and reloads them, and confirms equal values with tree-sitter-gram 0.7.0; if 0.7.0 does not read them back, the fix is a corpus case and a change in `gram-value`, not a local escape.

### 2.7 What stays

`Syntax`, `Span`, the diagnostics and their codes, the tag policy, identity and `@id` handling, the fixer, the record layout of the exporter and the load plan keep their shapes; they change only where they named `PropValue`.

## 3. Alternatives Considered

3.1. **Keep `PropValue` and call gram-value's writer only for export.** Rejected: the loader would keep a second model of every value, and each binding would still have to describe `PropValue` as well as `Value`. It saves the plumbing of 2.1 and leaves the divergence.

3.2. **A recursive `Located<Value>` tree with a span on every node.** Rejected: the only later uses of positions are a typed value's span and an array element's span (a database rejection of a tagged string, and `value.array-mixed`), which `span` and `items` cover. A full tree repeats the structure of `Value`.

3.3. **Use `gram_value::Map` for a record.** Rejected: a record needs the position of each key (for `key.duplicate`) and the order of writing, and the map is sorted and keeps neither.

3.4. **Write the canonical text but quote every string.** Rejected: it is a local rule on top of the shared writer, and the point of the shared text is that no tool carries a different one.

3.5. **Report a precision-losing decimal as an error or a warning now.** Not decided here: ADR-006 accepted such a literal and rounding it silently; the `precision` loss kind of `gram-value` is the place to report it, and the change affects released behavior. It is a separate decision (4).

3.6. **Adopt `Value` for the exporter only now and the loader later.** Rejected: the exporter's mapping from Relate values is the smaller half, but the loader's value type is what the bindings will meet first, and a release with two models gains little over one.

## 4. Consequences

- Library API changes, so the next release is a breaking minor version for library users (0.x): `values::PropValue` is removed, `typed::TypedValue`, `TypedContent` and `PointContent` are removed, `Tag::validate` returns a string, and `plan` structures hold `Located` values. The command line is unchanged. The changelog says so.
- Exported files change for every string property that is a symbol (bare instead of quoted); a user who diffs exports sees one change. Typed values and points are written as before. Releasing this together with typed tags (0.2.0) makes it the only export change users see.
- Code shrinks where the exporter's value writers go and grows slightly where `Located` is threaded through `plan.rs` and `load.rs`. The 122 contract tests, the round-trip and position tests are the safety net; the exporter's golden strings change.
- A point is interpreted twice (validation, then the write step). The performance test of ADR-012 (typed load of 10,000 elements within 1.2 times the earlier 416 ms) is re-run, and the step 2.3 is reconsidered if it exceeds that.
- Export bytes now follow `gram-value`'s canonical text, so a change of that text in a later minor version needs a deliberate bump and a look at the golden tests, as with the tags in ADR-012.
- The leading-slash escape is removed on the condition in 2.6; if tree-sitter-gram 0.7.0 does not read such strings back, the work moves to `gram-value`.
- Follow-up work: report a precision-losing decimal through gram-value's loss kinds (3.5); update `docs/supported-gram.md`, `README.md` and `specs/001-phase1-load-export/` where they show quoted strings; decide the bindings' value type on top of this one (open in `CLAUDE.md`); a Spec Kit specification and tasks (`specs/004-gram-value-model/`).

## 5. Related

- RFC-002 (property value mapping): goal 3.5 (one rule shared by loader, exporter and later bindings) and question 7.2 (canonical spelling).
- ADR-006 (property value mapping; its 2.5 string spelling and 2.6 export errors are amended here), ADR-011 (typed tags; 2.7 typed write path), ADR-012 (known tags from gram-value, the first step of this adoption), ADR-007 (the parser path that supplies the nodes).
- `gram-value`: https://github.com/gram-data/gram-value (its ADR-002 the kinds, ADR-003 the canonical text, ADR-005 the result shape).
