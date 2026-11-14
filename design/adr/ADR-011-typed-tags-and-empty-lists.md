---
id: ADR-011
title: Typed Tags, Typed Lists and Empty Lists
status: accepted
rfcs: [RFC-002]
created: 2026-11-13
---

# ADR-011: Typed Tags, Typed Lists and Empty Lists

> Amends ADR-006 sections 2.3 (tags), 2.5 and 2.6 (export), and resolves its deferred alternatives 3.2 and 3.3.

## 1. Context

Relate stores eight kinds of property value that gram has no literal for and that a person can read and write: dates, zoned and local times, zoned and local datetimes, durations, points and UUIDs. (It also stores vectors, which this decision leaves out; see 2.8.) A database that holds one cannot be exported today, because ADR-006 makes every such value an export error, and a gram file cannot say that a string is a date. Gram's tagged string (a tag symbol followed by a backtick-quoted string, as in ``date`2024-01-15` ``) is the notation's way to name a type for a string, and the loader currently drops every tag and reports the drop.

SPIKE-005 measured what Relate 1.0.1 does with these types:

- Each of these types is built from text by a constructor function (`date(text)`, `point({...})`, `uuid(text)`, ...), prints with `toString`, and reads back equal.
- Malformed text fails at construction, but Relate is lenient in places: `time('10:30:00')` and `datetime('2024-01-15')` are read as UTC without saying so, and a date can be written in a week form.
- Relate has no reader for a point's text; it has four spatial reference systems (4326 and 4979 geographic, 7203 and 9157 Cartesian) and rejects any other.
- `PointValue` is not exported by the `relate` crate, so a program outside Relate cannot build a point directly; it has to call the Cypher constructor.
- An empty list stores, with type `LIST<NOTHING>`.
- A map cannot be stored as a property, and `Bytes` is accepted only inside a statement.

Empty lists had a separate obstacle: tree-sitter-gram's array rule required at least one element, so `[]` was a syntax error until version 0.7.0 (requested upstream as gram-data/tree-sitter-gram#38).

Two properties of the work follow. `relate-gram check` reads no database, so a malformed date has to be found from the text alone. And the Cypher a load runs is built from batches of rows with a fixed statement per batch (ADR-001), while a typed value needs a constructor call in the statement.

## 2. Decision

### 2.1 Storable values and runtime values

A property value is storable when a database keeps it as a property: integer, float, boolean, string, a list of one of those, and the typed values of 2.2 and vectors. A runtime value can appear in a query result but cannot be stored as a property: a map, null, a node, a relationship, a path, and a byte array. Gram text can write only the storable values that a person can read and write, which excludes vectors (2.8). A conversion from a query result to gram (the exporter, or any later result-to-graph conversion) reports each runtime value it meets, with the element and key, and never writes it.

### 2.2 Tag vocabulary

A tagged string whose tag is one of the following, written in lower case, becomes a value of the matching Relate type. The names are the ones Relate's own constructors use. They do not overlap with the tags pattern-rs's exploratory schema contract uses for constraint dialects (`ts`, `re`, `zod`, `cypher`, `pydantic`, `gram`, in `specs/041-pato-cli/contracts/schema-gram.md`), which name validators and not value types. Any other tag keeps the behavior of ADR-006 2.3: the content loads as a string, and the drop is reported.

| Tag | Relate type | Content form |
|---|---|---|
| `date` | DATE | `YYYY-MM-DD`; the year has at least four digits and may be negative |
| `time` | ZONED TIME | `hh:mm:ss`, optional fraction of 1 to 9 digits, then `Z` or `+hh:mm` or `-hh:mm` |
| `localtime` | LOCAL TIME | `hh:mm:ss` with the optional fraction, no offset |
| `datetime` | ZONED DATETIME | a `date`, `T`, a `time` (offset required), and optionally a zone name in square brackets, as in `2024-01-15T10:30:00+01:00[Europe/Paris]`; the offset must be the zone's offset at that instant |
| `localdatetime` | LOCAL DATETIME | a `date`, `T`, a `localtime` |
| `duration` | DURATION | the ISO 8601 duration form, with the sign extension below, as Relate prints it: an upper-case `P`, then optional `nY`, `nM`, `nD`, then optionally `T` with `nH`, `nM`, `nS`, with at least one component; each number may carry a leading `-`; only the seconds may have a fraction, of up to 9 digits |
| `point` | POINT | extended well-known text, `SRID=n;POINT(x y)` for SRIDs 4326 and 7203, and `SRID=n;POINT Z(x y z)` for 4979 and 9157 (a 2D SRID with `POINT Z`, or the reverse, is an error); coordinates are decimal numbers without an exponent; for the geographic SRIDs `x` is longitude and `y` latitude |
| `uuid` | UUID | 32 hexadecimal digits in 8-4-4-4-12 groups, either case |

**Relation to ISO 8601.** The date, time, local time, local datetime and datetime forms are the extended-format ISO 8601 representations, restricted: seconds are required, second 60 and `24:00:00` are not accepted, and an offset is written `Z` or `±hh:mm`. Three forms go beyond ISO 8601, each because Relate stores or prints it that way:

- A year with a leading `-` (`-0044-03-15`) is an expanded year, which ISO 8601 allows only by prior agreement between the parties.
- A zone name in square brackets after a datetime's offset (`[Europe/Paris]`) is not ISO 8601; it is the notation of RFC 9557, the IETF extension of RFC 3339.
- A sign on each number of a duration (`P-1DT-2H`, or `P1DT-2H`) is not ISO 8601-1, which has no negative components. A duration holds months, days and nanoseconds separately, and their signs can differ, which a single leading minus cannot write. ISO 8601-2 allows that single leading minus on the whole duration; it is rejected here, since Relate prints the per-number form and one spelling is enough. The restrictions that remain, no weeks and a fraction on the seconds only, are narrower than ISO 8601, which allows both.

A point is extended well-known text, and a UUID is its canonical text; neither is covered by ISO 8601.

The loader accepts only these forms. The week, ordinal and basic (`20240115`) date spellings, a time without seconds, a time or datetime without an offset, a datetime written as a bare date, a duration in weeks or with a leading `-` before the `P`, and a point without an SRID are load errors, because Relate would either fill in a missing part or normalize the text, and a file that reads back different from what it wrote should not load silently. Hours are 0 to 23, minutes and seconds 0 to 59, months 1 to 12, a day exists in its month (leap years included), a geographic latitude is within [-90, 90], and coordinates are finite. A zone name must be non-empty and use only letters, digits and `/ _ + -`, which `check` verifies. `check` has no time-zone database, so whether the name is a known zone, and whether the offset is that zone's, is left to Relate, which rejects an unknown zone or an offset that is not the zone's when the value is built; the load reports that as in 2.3. Typed tags in the top-level record are validated like any value but are not stored, as for every record value (ADR-003, ADR-006 2.7).

### 2.3 Bad content is an error

A recognized tag whose content does not follow its form, or breaks a range above, is a load error `value.tag-content` at the tagged string's position, naming the tag and what is wrong with the text. It is never loaded as a plain string, since the tag names a type the value cannot have. `check` finds these errors without a database. If the database still rejects a value the loader accepted, the load reports it as the same error at the same position and changes nothing.

The `tags` setting (ADR-006 2.3) governs only unrecognized tags: `warn` drops and reports, `error` fails.

### 2.4 Typed lists

An array whose elements are all tagged strings with the same recognized tag loads as a list of that type. A recognized tag mixed with plain strings, or two different recognized tags in one array, is a load error `value.array-mixed`. Unrecognized tags in an array are dropped as before, and the array is a list of strings.

### 2.5 Empty lists

`[]` loads as an empty list, and an empty list exports as `[]`. tree-sitter-gram 0.7.0 accepts the form and the crate is pinned to it; this section records the behavior implemented with that bump, which landed before the rest of this decision.

### 2.6 Export

A typed value is written as a tagged string with the tag and the form of 2.2, so that exporting and reloading give an equal value. A datetime keeps its zone name when it has one, a duration is written in Relate's printed form, a point is written with its SRID and x, y (and z) as decimals without an exponent, a UUID in lower case. A list of one typed kind is written as an array of tagged strings. A list that mixes kinds, including Relate's mixed temporal lists, remains an export error (ADR-006 2.6). The other items of ADR-006 2.6 (non-finite floats, a string containing U+0000, an empty key) are unchanged; empty lists and typed values leave that list.

### 2.7 Building and reading values through Cypher

The loader writes typed values with a follow-up statement per (key, tag) pair in a batch, after the batch's other properties, built from the Cypher constructor that matches the tag and applied to the rows by internal id (for a list, to each element). A point is built from its parsed numbers and SRID. All statements run in the load's single transaction. The exporter reads typed values with Cypher accessors (`toString` for temporals and UUIDs; `.srid`, `.x`, `.y`, `.z` for points) and writes the forms of 2.2.

### 2.8 Vectors are not written

Gram text does not write vectors, in any form. A vector property is an export error that names the element and key (ADR-006 2.6 already lists it among values Relate may store but gram cannot hold), and no tag exists for it. A gram file cannot load one, so a round trip through gram never produces or drops a vector without saying so.

## 3. Alternatives Considered

3.1. **Pass typed values to Relate as parameters instead of constructing them in Cypher.** Rejected: `PointValue` is not exported by `relate`, so points cannot be built that way, and one route for every typed value is simpler than a mix.

3.2. **Fall back to a plain string when the content of a recognized tag does not parse.** Rejected: the file names a type and the database would hold a different one, which is the silent change Constitution II forbids (nothing is dropped or altered without a report); the error costs a rewrite of one value.

3.3. **Accept whatever forms Relate's constructors accept.** Rejected: Relate reads an offsetless datetime as UTC and a bare date as a datetime, so `check` could not find these without a database and the stored value would differ from the text.

3.4. **Validate content by running Relate's constructors in a scratch in-memory database during `check`.** Rejected for the forms Relate is lenient about (it reads an offsetless datetime as UTC and a bare date as a datetime), which the loader must reject on its own, and because it makes `check` start a database engine. The one thing this route would give is validation of zone names, which needs the time-zone database; 2.2 leaves zone names to the load instead.

3.5. **Write a point as Relate prints it, `point({x: 1.0, y: 2.0, crs: 'cartesian'})`.** Rejected: it is a Cypher expression, not a text a gram reader or a GIS tool understands; extended well-known text is the interchange form for points, and the loader maps it onto Relate's four SRIDs.

3.6. **A tag for vectors.** Rejected: a vector is a long list of numbers (an embedding has hundreds or thousands), which a person cannot meaningfully read or write, and readable, writable text is what gram is for. Vectors stay unwritable: the export fails and lists each vector (2.8).

3.7. **A tag per Relate type name, including `zoned time` and `zoned datetime`.** Rejected: a tag is a gram symbol, and a space is not allowed in one; `time` and `datetime` are the names Relate's own constructors use.

3.8. **Write `[]` before the grammar accepted it.** Rejected while the grammar was at 0.6.2: an export that the project's own parser rejects is a defect (the round-trip property of ADR-006). The grammar bump removed the obstacle.

## 4. Consequences

- A database holding any of the eight types, or an empty list, can now be exported once the work is built, and a gram file can state a date, a time, a duration, a point or a UUID. A file that already used one of these tags will change meaning: strings that were dropped-and-reported now become typed values, or fail with `value.tag-content`. The changelog says so.
- `check` cannot find an unknown zone name or an offset that is not the zone's: the load reports it, at the tagged string's position, only when a database is written to.
- The loader owns a validator for each form of 2.2. Relate's constructors still run, so an inconsistency between the two is a possible but rare load error; tests cover each form against Relate.
- Points exist only as extended well-known text and only for Relate's four SRIDs; a point in any other reference system cannot be written.
- The write path gains one statement per (key, tag) pair per batch. The cost is small for the usual file and grows with the number of distinct typed keys in a batch.
- A mixed-kind list, a map, null, a node, a relationship, a path and a byte array remain unwritable. A result conversion must report them rather than skip them.
- A database with vector properties still cannot be exported while those properties are present. A person can remove them or move them elsewhere, and the error lists each one.
- The fixer (ADR-010) has no fix for a bad typed value, since it cannot know the intended value, and `--tags error` makes no difference to recognized tags: they are always typed.
- The empty-list behavior (2.5) was implemented, with the grammar bump, before the typed tags; the typed tags, `value.tag-content` and the follow-up statements are not yet built.
- Follow-up work: the new diagnostic codes (`value.tag-content`, and `value.array-mixed` for tags) go in the diagnostics contract and `docs/supported-gram.md`; the tag vocabulary is a candidate for the gram specification, and RFC-002 questions 7.3 and 7.4 are resolved by this decision, except that vectors and byte arrays, which 7.3 lists, get no tag (2.1, 2.8).

## 5. Related

- RFCs: RFC-002 (resolves questions 7.3 and 7.4 and, for empty lists, the empty-list part of 7.5; the pattern-rs coordination 7.3 asks for is addressed in 2.2).
- ADR-006 is amended as stated above; ADR-007 (the syntax tree the tags are read from) is unchanged.
- SPIKE-005 records how Relate 1.0.1 stores, prints and round-trips each type.
- Upstream: gram-data/tree-sitter-gram#38 (allowed `[]`; released in 0.7.0).
