# Research: gram-value as the Value Model

## R1. Constructing values in the lowering

- **Decision**: decimal integers: text `-0` becomes `Value::integer(0)`; other text goes to `Value::integer_from_digits`, and a `BigInteger` result is `value.integer-range`. Hexadecimal integers are parsed to `i64` locally. Decimals go to `Value::decimal_from_text`; a `BigDecimal` result is re-parsed from `as_str()` as `f64` and the existing finite and non-zero checks apply. Strings, booleans and arrays use `Value::string`, `Value::boolean`, `Value::array`; a recognized tag uses `Value::tagged(tag, normal_content)`.
- **Rationale**: ADR-013 section 2.2; a throwaway run showed `integer_from_digits("-0")` is an error while the grammar accepts `-0`.
- **Alternatives**: parse everything locally (rejected: loses the single definition of kinds).

## R2. Positions

- **Decision**: a small struct `Lowered { value, span, items }` where `items` holds the element spans of an array. The typed write step and `typed_failure` read `span` and `items`; `value.array-mixed` is raised during lowering from the element nodes.
- **Rationale**: ADR-013 section 2.1; the reviewer verified no later step uses positions of nested parts.

## R3. Typed write step

- **Decision**: `typed_tag`, `typed_param`, `group_spans` and `typed_text` read the tag and content from `Value::Tagged` (`Value::tag()`, `Value::content()` through the `Tagged` accessors). For a point, `typed_param` calls `gram_value::tags::interpret_tag("point", content)` and builds the same parameter map as now. `typed_text` returns the content for every tag, including points.
- **Rationale**: ADR-013 section 2.3; matching points in `typed_failure` becomes possible and is intended.

## R4. Exporter mapping

- **Decision**: a function from a Relate property value to `Result<gram_value::Value, String>`: integer, boolean, string (NUL is an error), float (finite, else error; `-0.0` accepted), list (empty gives `Value::array([])`; nested and mixed lists are errors checked first), UUID and point as tagged values, temporal values as tagged values with Relate's text. The record text keeps `names::name(key)` and joins `key: <to_text()>`.
- **Rationale**: ADR-013 section 2.5; reviewer verified floats, escapes, empty strings, digit-first and non-ASCII strings agree, and that the symbol rule equals tree-sitter-gram's.
- **Alternatives**: `Value::map` for the whole record (rejected: keys are written differently, ADR-013 section 2.5).

## R5. The leading-slash escape

- **Decision**: remove the escape for string values only; `escape_into` and its use for backtick-quoted names stay. A deterministic test over `//`, `/** ... */` and blanks-then-slash as value, key and identifier decides it; failure keeps the escape.
- **Rationale**: ADR-013 section 2.6; reviewer loaded such strings, keys and names with 0.7.0 successfully, and `tests/fixtures/valid/slashes.gram` covers load.

## R6. Performance baseline

- **Decision**: T001 re-runs the release perf test to confirm 348.7 ms still holds on this machine; the ceiling is 1.2 times the figure measured then if it differs by more than 10%.
- **Rationale**: SC-004 is relative to the state before this change.
