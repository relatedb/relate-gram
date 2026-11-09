---
id: SPIKE-002
title: Relate Property Value Types
status: done
rfcs: [RFC-002]
created: 2026-11-08
---

# SPIKE-002: Relate Property Value Types

## 1. Question

Which values can Relate (crate `relate`, 0.16.1) store as node and relationship properties, and how does it treat the edge cases that gram can produce or that an export must handle? Specifically: integer range and overflow, float spellings and non-finite values, string content, list typing (mixed, empty, nested), null, the non-basic types (temporal, point, vector, UUID, bytes), and property keys. Informs RFC-002 questions 7.1, 7.3, 7.5 and 7.10.

## 2. Method

A throwaway Rust binary, `scripts/probe`, depends on the public `relate` crate by path and runs literal Cypher and parameter-supplied values against a fresh in-memory database, printing each result or error. Run with `cargo run --release --manifest-path design/spikes/SPIKE-002-relate-property-values/scripts/probe/Cargo.toml`. Output: `probe-output.txt`.

## 3. Time-box

Two hours.

## 4. Findings

- **Integers** are 64-bit signed, with `i64::MIN` and `i64::MAX` stored exactly. A literal one above the maximum is a syntax error (`IntegerOverflow`).
- **Floats** store `1.0`, `0.1`, `1e300` and `-0.0` exactly. `NaN`, `+Infinity` and `-Infinity` are storable when supplied as parameters. Gram's `decimal` has no exponent and no non-finite spelling.
- **Strings** keep escapes, accents, emoji and the empty string. A string with NUL and other control characters is storable.
- **Lists** of integers, strings, booleans and floats are stored. An empty list is stored as `LIST<NOTHING>`. A list mixing integers and floats is **silently coerced** to floats (`[1, 2.5]` reads back `[1.0, 2.5]`). A list mixing other types, a list containing null, a nested list and a map are all `InvalidPropertyType` errors.
- **Null:** setting a property to null stores nothing (the key is absent).
- **Non-basic types** are storable and read back typed: date, datetime, localdatetime, time, localtime, duration (`toString` gives ISO-8601 forms), points (with an SRID, 7203 for Cartesian and 4326 for WGS-84), lists of dates, and vectors. A UUID from `randomUUID()` is a string. Bytes are rejected as a property (`TypeError`).
- **Keys** may contain spaces, dots, non-ASCII characters, or be empty. A repeated key in one map literal keeps one value (the last) without an error.

## 5. Conclusion

Relate stores everything gram's scalar values can become, with `i64` and `f64` as the numeric limits. Two behaviors need loader rules because the database will not enforce them: it coerces mixed integer and float lists without saying so, and it keeps the last of a repeated key. Values such as dates and points can be stored but have no gram literal, so their export needs a tag vocabulary (RFC-002 question 7.3), which this spike does not settle.
