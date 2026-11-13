---
id: SPIKE-005
title: Relate's Typed Property Values and Their String Forms
status: done
rfcs: [RFC-002]
created: 2026-11-13
---

# SPIKE-005: Relate's Typed Property Values and Their String Forms

## 1. Question

RFC-002 questions 7.3 and 7.4: for each non-basic value type Relate 1.0.1 stores as a property (date, time, local time, datetime, local datetime, duration, point, vector, uuid), what does it store, what does it return and print, and does the printed text read back to an equal value? The answer sets the tag vocabulary and textual forms for typed gram tags, and tells whether a tag whose content does not parse can be reported by the loader before the database sees it.

## 2. Method

A throwaway program under `scripts/probe` depends on `relate` 1.0.1 from crates.io (no default features) and runs Cypher statements against an in-memory database. For each type it creates a property from a string constructor, reads it back as a `Value`, prints it with `toString`, rebuilds a value from that text, and compares the two. It also tries edge forms (offsets, zone names, fractional seconds, negative and mixed durations, 3D and geographic points, vectors of each coordinate kind), lists of typed values, malformed text, and `valueType` for an empty list. The full output is `probe-output.txt`.

## 3. Time-box

Half a day.

## 4. Notes

2026-11-13
- Vectors were dropped from the typed-tag vocabulary after the spike (ADR-011 2.8): a vector is a very long list of numbers that a person cannot read or write, so gram does not carry it. The vector findings below stay as evidence.
- Added during the ADR-011 review (`== review probes` in the output): Relate rejects a zone name that does not match the offset (`+05:00[Europe/Paris]`, and `Z[Europe/Paris]`) and an unknown zone (`[Mars/Base]`), so zone names need the time-zone database to validate. It is more lenient than the ADR's forms in other ways: `time('10:30')` (no seconds) and `time('10:30:60Z')` (second 60) are accepted, `duration('P1.5D')` becomes 1 day 12 hours, `duration('p1d')` (lower case) and `duration('P1W2D')` are accepted, `date('+2024-01-15')` and `date('20240115')` are accepted, and a fraction of ten digits is truncated to nine. It rejects `2023-02-29`, a UUID without hyphens or in braces, and a point whose SRID does not match its dimension (`z` with 4326, or no `z` with 4979); coordinates may be written in exponent form (`1e3`).
- `Value` has the variants `Temporal`, `Point`, `Uuid` and `Vector` for the types in question, and also `Bytes`, which the engine documents as statement-scoped: it can appear in results but storing it as a property fails. `Temporal` and `Value` are re-exported by the `relate` crate; the point and vector types are not, so a loader outside Relate cannot build those values directly and has to construct them with Cypher (`date($s)`, `point({...})`). A constructor used with a parameter works inside `CREATE` (`CREATE (n {d: date($s)})` stored a date).

## 5. Findings

Probe: `scripts/probe`, output: `probe-output.txt` (Relate 1.0.1, in-memory).

**Stored types and their text** (`toString`, which reads back equal in every case tried):

| Type (`valueType`) | Constructor | Printed form | Notes |
|---|---|---|---|
| `DATE` | `date(text)` | `2024-01-15` | Also reads `2024-W03-1`; prints the calendar form. Negative years work (`-0044-03-15`). A datetime string is rejected. |
| `ZONED TIME` | `time(text)` | `10:30:00+01:00`, `10:30:00Z`, `10:30:00.123456789+05:30` | Offset in seconds, nanosecond precision. Text without an offset is read as UTC. |
| `LOCAL TIME` | `localtime(text)` | `10:30:00`, `10:30:00.123456789` | |
| `ZONED DATETIME` | `datetime(text)` | `2024-01-15T10:30:00Z`, `...+01:00`, `...+01:00[Europe/Paris]` | Keeps a zone name when given and prints the offset with it. Text without an offset, or a bare date, is read as UTC, so a missing offset is silently filled in. |
| `LOCAL DATETIME` | `localdatetime(text)` | `2024-01-15T10:30:00`, with nanoseconds | |
| `DURATION` | `duration(text)` | `P1DT2H`, `P1Y2M3DT4H5M6.789S`, `P-1DT-2H` | Months, days and nanoseconds are kept apart. Weeks become days (`P2W` prints `P14D`). A negative duration prints with a sign on each component and reads back equal. |
| `POINT` | `point({x, y[, z], srid or crs})` | `point({x: 1.0, y: 2.0, crs: 'cartesian'})` | Only four SRIDs exist: 7203 (cartesian), 9157 (cartesian 3D), 4326 (WGS-84), 4979 (WGS-84 3D); any other is an error. Geographic points store longitude as `x` and latitude as `y`; latitude outside [-90, 90] is an error. Relate has no WKT reader (`point('POINT(1 2)')` is an error) and prints no WKT. |
| `VECTOR<kind>(n)` | `vector(list, n, kind)` | `vector([1.0, 2.0, 3.0], 3, FLOAT32)` | A coordinate kind (`FLOAT32`, `INTEGER8`, ...) and a dimension are part of the value. `vector('[1,2,3]', 3, FLOAT32)` also works. |
| `UUID` | `uuid(text)` | `123e4567-e89b-12d3-a456-426614174000` | Stored as 128 bits, printed lowercase; uppercase input reads equal. `randomUUID()` returns a string, not a UUID value. |

**Bad text fails at construction**, with a message naming the text: `date('not a date')`, `date('2024-13-45')`, `time('25:00')`, `duration('1 day')`, `uuid('xyz')`, `point({x: 1.0})`. Two leniencies: a datetime or time without an offset is accepted as UTC.

**Lists.** A list of one typed kind stores (`LIST<DATE NOT NULL>`, `LIST<DURATION ...>`, `LIST<POINT ...>`). Relate also stores a list mixing temporal kinds (`LIST<DATE | ZONED DATETIME>`). An empty list stores, with type `LIST<NOTHING>`.

**Runtime-only values.** A map as a property value is an error (`InvalidPropertyType`); setting a property to null stores nothing. `Bytes` is statement-scoped. This matches the line between storable and runtime values drawn in RFC-005.

## 6. Conclusion

All nine types round-trip through a string form, and Relate rejects malformed text at construction, so a loader can pass the tag's content to the constructor and report a failure as a located load error (RFC-002 question 7.4: an error, not a fallback to a plain string, since the type is named).

- The tag vocabulary can follow Relate's `valueType` names: `date`, `time`, `localtime`, `datetime`, `localdatetime`, `duration`, `point`, `vector`, `uuid`. The textual forms are the printed forms above (ISO 8601 for the temporal types, including the zone name in brackets for `datetime`).
- **Point has no standard text in Relate.** WKT with an SRID (`SRID=4326;POINT(-74.0 40.7)`) would have to be written and read by this project's own code, mapped to Relate's four SRIDs, with longitude first. Alternatively the tag could carry Relate's own `point({...})` text form, which has to be parsed here too. The ADR must pick one.
- **Vector** needs a kind and dimension in its text; gram's tagged string has no place for them except inside the content.
- **Leniency**: an offsetless `time` or `datetime` becomes UTC. The loader should require the form to match the tag (for example, `datetime` must carry an offset) so an offsetless value is not silently read as UTC, while `localdatetime` and `localtime` take no offset.
- **Construction**: values must be built in Cypher (constructor functions with parameters); `PointValue` and the vector coordinate kind are not in `relate`'s public API, so they cannot be built directly. This affects how the loader batches writes (ADR-001 section 2.5) and has to be settled in the ADR.

Text for RFC-002 question 7.3 and 7.4: "Answered by SPIKE-005: the vocabulary follows Relate's value types, texts round-trip, bad content fails at construction; point text and vector parameters need a decision."
