---
id: ADR-011
title: Typed Tags, Typed Lists and Empty Lists
status: proposed
rfcs: [RFC-002]
created: 2026-11-13
---

# ADR-011: Typed Tags, Typed Lists and Empty Lists

> Amends ADR-006 sections 2.3 (tags), 2.5 and 2.6 (export), and resolves its deferred alternatives 3.2 and 3.3.

## 1. Context

Relate stores nine kinds of property value that gram has no literal for: dates, zoned and local times, zoned and local datetimes, durations, points, vectors and UUIDs. A database that holds one cannot be exported today, because ADR-006 makes every such value an export error, and a gram file cannot say that a string is a date. Gram's tagged string (a tag symbol followed by a backtick-quoted string, as in ``date`2024-01-15` ``) is the notation's way to name a type for a string, and the loader currently drops every tag and reports the drop.

SPIKE-005 measured what Relate 1.0.1 does with these types:

- Each type is built from text by a constructor function (`date(text)`, `point({...})`, `vector(list, n, kind)`, ...), prints with `toString`, and reads back equal.
- Malformed text fails at construction, but Relate is lenient in places: `time('10:30:00')` and `datetime('2024-01-15')` are read as UTC without saying so, and a date can be written in a week form.
- Relate has no reader for a point's text; it has four spatial reference systems (4326 and 4979 geographic, 7203 and 9157 Cartesian) and rejects any other.
- A vector is a list of numbers with a coordinate kind (`INTEGER8` to `INTEGER64`, `FLOAT16`, `BFLOAT16`, `FLOAT32`, `FLOAT64`).
- `PointValue` and the vector coordinate kind are not exported by the `relate` crate, so a program outside Relate cannot build those values; it has to call Cypher constructors.
- An empty list stores, with type `LIST<NOTHING>`.
- A map cannot be stored as a property, and `Bytes` is accepted only inside a statement.

Empty lists have a separate obstacle: tree-sitter-gram's array rule requires at least one element, so `[]` is a syntax error (reported upstream as gram-data/tree-sitter-gram#38).

Two properties of the work follow. `relate-gram check` reads no database, so a malformed date has to be found from the text alone. And the Cypher a load runs is built from batches of rows with a fixed statement per batch (ADR-001), while a typed value needs a constructor call in the statement.

## 2. Decision

### 2.1 Storable values and runtime values

A property value is storable when a database keeps it as a property: integer, float, boolean, string, a list of one of those, and the typed values of 2.2. A runtime value can appear in a query result but cannot be stored as a property: a map, null, a node, a relationship, a path, and a byte array. Gram text can write only storable values. A conversion from a query result to gram (the exporter, or any later result-to-graph conversion) reports each runtime value it meets, with the element and key, and never writes it.

### 2.2 Tag vocabulary

A tagged string whose tag is one of the following, written in lower case, becomes a value of the matching Relate type. Any other tag keeps the behavior of ADR-006 2.3: the content loads as a string, and the drop is reported.

| Tag | Relate type | Content form |
|---|---|---|
| `date` | DATE | `YYYY-MM-DD`; the year has at least four digits and may be negative |
| `time` | ZONED TIME | `hh:mm:ss`, optional fraction of 1 to 9 digits, then `Z` or `+hh:mm` or `-hh:mm` |
| `localtime` | LOCAL TIME | `hh:mm:ss` with the optional fraction, no offset |
| `datetime` | ZONED DATETIME | a `date`, `T`, a `time` (offset required), and optionally a zone name in square brackets, as in `2024-01-15T10:30:00+01:00[Europe/Paris]` |
| `localdatetime` | LOCAL DATETIME | a `date`, `T`, a `localtime` |
| `duration` | DURATION | ISO 8601 as Relate prints it: `P`, then optional `nY`, `nM`, `nD`, then optionally `T` with `nH`, `nM`, `nS`; each number may carry a leading `-`, and seconds may have a fraction of up to 9 digits |
| `point` | POINT | extended well-known text, `SRID=n;POINT(x y)` for SRIDs 4326 and 7203, and `SRID=n;POINT Z(x y z)` for 4979 and 9157; for the geographic SRIDs `x` is longitude and `y` latitude |
| `vector` | VECTOR | a coordinate kind in upper case, then the elements in square brackets: `FLOAT32[1.0, 2.0, 3.0]`, `INTEGER8[1, 2, 3]`; the dimension is the number of elements |
| `uuid` | UUID | 32 hexadecimal digits in 8-4-4-4-12 groups, either case |

The loader accepts only these forms. The week and ordinal date spellings, a time or datetime without an offset, a datetime written as a bare date, a duration in weeks, and a point without an SRID are load errors, because Relate would either fill in a missing part or normalize the text, and a file that reads back different from what it wrote should not load silently. Hours are 0 to 23, minutes and seconds 0 to 59, months 1 to 12, a day exists in its month (leap years included), a geographic latitude is within [-90, 90], coordinates are finite, and a vector's elements are all integers (within the kind's range) for the integer kinds and all numbers for the float kinds.

### 2.3 Bad content is an error

A recognized tag whose content does not follow its form, or breaks a range above, is a load error `value.tag-content` at the tagged string's position, naming the tag and what is wrong with the text. It is never loaded as a plain string, since the tag names a type the value cannot have. `check` finds these errors without a database. If the database still rejects a value the loader accepted, the load reports it as the same error at the same position and changes nothing.

The `tags` setting (ADR-006 2.3) governs only unrecognized tags: `warn` drops and reports, `error` fails.

### 2.4 Typed lists

An array whose elements are all tagged strings with the same recognized tag loads as a list of that type. A recognized tag mixed with plain strings, or two different recognized tags in one array, is a load error `value.array-mixed`. Unrecognized tags in an array are dropped as before, and the array is a list of strings.

### 2.5 Empty lists

`[]` loads as an empty list, and an empty list exports as `[]`. This takes effect when the pinned tree-sitter-gram version accepts the form; until then an empty list stays an export error, and the grammar version bump is the change that turns the feature on, with a corpus case.

### 2.6 Export

A typed value is written as a tagged string with the tag and the form of 2.2, so that exporting and reloading give an equal value. A datetime keeps its zone name when it has one, a duration is written in Relate's printed form, a point is written with its SRID and x, y (and z), a vector with its kind, a UUID in lower case. A list of one typed kind is written as an array of tagged strings. A list that mixes kinds, including Relate's mixed temporal lists, remains an export error (ADR-006 2.6). The other items of ADR-006 2.6 (non-finite floats, a string containing U+0000, an empty key) are unchanged; empty lists and typed values leave that list.

### 2.7 Building and reading values through Cypher

The loader writes typed values with a follow-up statement per (key, tag) pair in a batch, after the batch's other properties: `UNWIND $rows AS r MATCH (n) WHERE id(n) = r.id SET n.\`key\` = date(r.v)`, with the constructor chosen by the tag, `[x IN r.v | date(x)]` for a list, and the vector's kind and dimension as literals in the statement (rows are grouped by them). A point is built from its parsed numbers, `point({x: r.x, y: r.y, srid: r.srid})`. All statements run in the load's single transaction. The exporter reads typed values with Cypher accessors (`toString` for temporals, UUIDs and vectors; `.srid`, `.x`, `.y`, `.z` for points) and writes the forms of 2.2.

## 3. Alternatives Considered

3.1. **Pass typed values to Relate as parameters instead of constructing them in Cypher.** Rejected: `PointValue` and the vector coordinate kind are not exported by `relate`, so points and vectors cannot be built that way, and one route for all nine types is simpler than a mix.

3.2. **Fall back to a plain string when the content of a recognized tag does not parse.** Rejected: the file names a type and the database would hold a different one, which is the silent change Constitution II forbids; the error costs a rewrite of one value.

3.3. **Accept whatever forms Relate's constructors accept.** Rejected: Relate reads an offsetless datetime as UTC and a bare date as a datetime, so `check` could not find these without a database and the stored value would differ from the text.

3.4. **Write a point as Relate prints it, `point({x: 1.0, y: 2.0, crs: 'cartesian'})`.** Rejected: it is a Cypher expression, not a text a gram reader or a GIS tool understands; extended well-known text is the interchange form for points, and the loader maps it onto Relate's four SRIDs.

3.5. **Encode a vector's elements as a gram array and its kind elsewhere.** Rejected: a tagged string is one value; splitting it across a property and a second property or tag would break the single-property rule of ADR-006.

3.6. **A tag per Relate type name, including `zoned time` and `zoned datetime`.** Rejected: a tag is a gram symbol, and a space is not allowed in one; `time` and `datetime` are the names Relate's own constructors use.

3.7. **Support empty lists by writing `[]` before the grammar accepts it.** Rejected: an export that the project's own parser rejects is a defect (the round-trip property of ADR-006).

## 4. Consequences

- A database holding any of the nine types, or an empty list, can now be exported once the work is built, and a gram file can state a date, a point, a vector or a UUID. A file that already used one of these tags will change meaning: strings that were dropped-and-reported now become typed values, or fail with `value.tag-content`. The changelog says so.
- The loader owns a validator for each form of 2.2. Relate's constructors still run, so an inconsistency between the two is a possible but rare load error; tests cover each form against Relate.
- Points exist only as extended well-known text and only for Relate's four SRIDs; a point in any other reference system cannot be written.
- The write path gains one statement per (key, tag) pair per batch. The cost is small for the usual file and grows with the number of distinct typed keys in a batch.
- A mixed-kind list, a map, null, a node, a relationship, a path and a byte array remain unwritable. A result conversion must report them rather than skip them.
- The vector form is chosen here, with no external standard behind it, and may need revision if gram's own tag conventions settle on another.
- Follow-up work: the new diagnostic codes (`value.tag-content`, and `value.array-mixed` for tags) go in the diagnostics contract and `docs/supported-gram.md`; the fixer has no fix for a bad typed value (it cannot guess the intent); the tag vocabulary is a candidate for the gram specification, and RFC-002 questions 7.3 and 7.4 are resolved by this decision; upstream, tree-sitter-gram must release the empty-array rule.

## 5. Related

- RFCs: RFC-002 (resolves questions 7.3, 7.4 and, for empty lists, the empty-list part of 7.5).
- ADR-006 is amended as stated above; ADR-007 (the syntax tree the tags are read from) is unchanged.
- SPIKE-005 records how Relate 1.0.1 stores, prints and round-trips each type.
- Upstream: gram-data/tree-sitter-gram#38 (allow `[]`).
