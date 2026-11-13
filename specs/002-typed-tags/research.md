# Research: Typed Tags for Gram Values

Decisions that ADR-011 leaves to implementation, each checked against Relate 1.0.1 (SPIKE-005 and a probe run for this plan).

## R1. How typed values are read on export

- **Decision**: read `properties(n)` as today. A UUID is formatted with `relate::uuid_hyphenated`; a point with the public fields `srid`, `x`, `y`, `z` of the matched `Value::Point`. Temporal values (and lists of them) take their text from a column added to the two existing export queries: `[k IN keys(n) WHERE <temporal> | [k, toString(n[k])]]`, with `[x IN n[k] | toString(x)]` for lists.
- **Rationale**: `Temporal`'s `Display` and `iso_string()` drop zero seconds (`2024-01-15T10:30+01:00[Europe/Paris]`), which is not the `hh:mm:ss` form ADR-011 section 2.2 requires, while Cypher's `toString` prints `10:30:00`. Dynamic property access (`n[k]`) and `valueType` work in Relate 1.0.1.
- **Alternatives considered**: format the `Temporal` variants ourselves (needs the time library's types, which `relate` does not re-export); post-process `Display` text (fragile); one extra query per element (slow).

## R2. How typed values are written

- **Decision**: after the basic properties of a batch, run one statement per (element kind, key, tag, shape) group, by internal id: `... SET n.\`key\` = date(r.v)`, `[x IN r.v | date(x)]` for a list, `point({x: r.x, y: r.y, srid: r.srid})` for 2D and with `z: r.z` for 3D (the shape). All run in the load's transaction. Matched elements are updated the same way, so the file's value wins.
- **Rationale**: all of these statements ran against 1.0.1 (nodes, relationships, lists, both point shapes, UUID with upper-case input, negative duration, datetime with zone); the database rejects bad content with `type error: invalid date '...'`, which maps to `value.tag-content` at the tagged string's position. `PointValue` is not re-exported, so a point cannot be passed as a parameter.
- **Alternatives considered**: pass `Value::Temporal`/`Value::Uuid` as parameters and use Cypher only for points (two routes; ADR-011 alternative 3.1); inline literals in the statement text (quoting and injection risk).

## R3. The content-form validators

- **Decision**: a hand-written validator per tag in `typed`, returning either a normalized text (the form passed to the constructor) or a located fault message; ranges from ADR-011 section 2.2 (calendar days with leap years, hours to 23, minutes and seconds to 59, latitude within 90, finite coordinates, SRID and dimension agree). Zone names are copied through unchecked.
- **Rationale**: ADR-011 alternative 3.4 rejects delegating to a database; the forms are short; SPIKE-005 lists the places Relate is more lenient (`P1.5D`, lower-case `p1d`, `time('10:30')`, second 60, basic dates), each of which gets a rejecting test.
- **Alternatives considered**: a date/time crate for validation (a dependency for a handful of range checks, and its idea of valid text would not match Relate's).

## R4. Typed values and identity

- **Decision**: a typed value is not usable as an identity, exactly like a list: with an identity property configured, an element whose identity property is typed is treated as lacking it (`id.property-missing`), and a typed `@id` is impossible (`@`-keys are errors).
- **Rationale**: `plan::value_text` already returns `None` for every value that is not a string or integer; the new variant falls under the same rule with no new code path, which resolves the edge case the spec marked for planning.
- **Alternatives considered**: use the tag's text as identity (a date is a poor key and would need its own ADR).

## R5. Dependencies

- **Decision**: none added.
- **Rationale**: the validators need only integer parsing and calendar arithmetic (a leap-year rule); the UUID check is the 8-4-4-4-12 hexadecimal pattern, and `relate::uuid_parse` is available for tests.
- **Alternatives considered**: `time`, `chrono`, `uuid` crates (see R3).

## R6. Library interface change

- **Decision**: `values::PropValue` gains a variant; the crate is pre-1.0, so this is allowed by the versioning policy ("a minor version may change the library interface"). The changelog and `contracts/library-api.md` say so.
- **Alternatives considered**: `#[non_exhaustive]` now (a breaking change in itself, applied once, would stop future ones); a separate type for typed values (two parallel value types through load and plan).
