# Feature Specification: Typed Tags for Gram Values

**Feature Branch**: `typed-tags-spec`

**Created**: 2026-11-13

**Status**: Draft

**Input**: User description: "typed tags: implement ADR-011 (typed tags for date, time, localtime, datetime, localdatetime, duration, point, uuid; typed lists; export of typed values as tagged strings; vectors stay unwritable; empty lists already done)."

This specification implements ADR-011 (accepted). It names the sections of ADR-011 and ADR-006 that decide each behavior and does not restate them; where they differ, the ADR wins. Empty lists, the other half of ADR-011, shipped with the tree-sitter-gram 0.7.0 bump and are not part of this feature.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - A tagged string becomes a typed value on load (Priority: P1)

A developer has a gram file in which some properties are dates, times, durations, points or UUIDs, written as tagged strings such as ``{born: date`1990-04-12`}``. They load the file and the database stores each of those properties as a value of the matching Relate type, so that Cypher date arithmetic, point distance and UUID comparison work on them.

**Why this priority**: This is the reason for the feature. Until a tag means something, a date in a gram file is a string.

**Independent Test**: Load a file with one property of each of the eight tags into an empty database, then ask the database for the type of each stored value and compare the values with what the file says.

**Acceptance Scenarios**:

1. **Given** a file with a `date`, `time`, `localtime`, `datetime`, `localdatetime`, `duration`, `point` and `uuid` tagged string, each in a form ADR-011 section 2.2 lists, **When** it is loaded, **Then** each property is stored as a value of the matching type (DATE, ZONED TIME, LOCAL TIME, ZONED DATETIME, LOCAL DATETIME, DURATION, POINT, UUID) and equals the value the text names.
2. **Given** a file whose tag is not in the vocabulary (for example `md` or `Date`), **When** it is loaded, **Then** the content is stored as a string and the drop is reported, exactly as before (ADR-006 section 2.3, ADR-011 section 2.2).
3. **Given** typed tags on node properties, relationship properties and in the top-level record, **When** the file is loaded, **Then** node and relationship properties are typed and record values are validated but not stored (ADR-011 section 2.2, ADR-006 section 2.7).
4. **Given** a `datetime` with a zone name, **When** it is loaded and queried, **Then** the stored value keeps the zone name and offset.

---

### User Story 2 - A bad value is rejected with a position (Priority: P1)

A developer writes ``date`2024-13-45` ``, a time with no offset, or a point with an unsupported reference system. The load fails, leaves the database unchanged, and says which tag, which text and where in the file, so they can correct the file. `check` finds the same errors without a database.

**Why this priority**: The strictness rule (Constitution II) means a recognized tag never loads as something else; an unchecked date would be worse than no date support.

**Independent Test**: Load and check one invalid file per rejected form (ADR-011 section 2.2 lists them), each against a database with known content, and verify the code, the position and that the database is unchanged.

**Acceptance Scenarios**:

1. **Given** a recognized tag whose content breaks its form or a range (a month 13, a day that does not exist in its month, an hour 24, a latitude above 90, a non-finite coordinate, a point SRID that does not match the number of coordinates), **When** it is loaded or checked, **Then** it fails with the `value.tag-content` code at the tagged string's position, naming the tag and what is wrong (ADR-011 section 2.3).
2. **Given** content in a form Relate would accept but ADR-011 rejects (a time or datetime with no offset, a bare date as a datetime, a week or basic date, a duration in weeks, a duration with a leading `-`, a point without an SRID), **When** it is loaded or checked, **Then** it fails as in scenario 1 (ADR-011 section 2.2).
3. **Given** a `datetime` whose zone name is unknown, or whose offset is not the zone's, **When** it is checked, **Then** `check` reports no error for the zone name, **and When** it is loaded, **Then** the load fails at the same position and the database is unchanged (ADR-011 section 2.2 and 2.3).
4. **Given** several bad values in one file, **When** it is checked or loaded, **Then** every one is reported with its position, up to the existing cap, with the total.

---

### User Story 3 - Typed values export as tagged strings and round-trip (Priority: P1)

A developer has a Relate database holding dates, times, durations, points and UUIDs. They export it as gram and get a file that states those values as tagged strings. Loading that file into an empty database gives equal values.

**Why this priority**: Round-trip fidelity (Constitution IV) is the other half of the promise; today any such database fails to export.

**Independent Test**: Build a database with each type, export it, load the export into a fresh database, and compare every property's type and value.

**Acceptance Scenarios**:

1. **Given** a database with a property of each of the eight types, **When** it is exported, **Then** each is written as a tagged string with the tag and form of ADR-011 section 2.2 and 2.6, and the export of the same database is identical every time.
2. **Given** the exported file, **When** it is loaded into an empty database, **Then** each value has the same type and equals the original (including nanoseconds, a negative duration, a three-dimensional point and a datetime with a zone name).
3. **Given** a geographic point, **When** it is exported, **Then** the text has longitude first and the SRID Relate stores (ADR-011 section 2.2).

---

### User Story 4 - Lists of typed values (Priority: P2)

A developer has a property that is a list of dates or of points. It loads from an array of tagged strings with one tag and exports back as that array.

**Why this priority**: Lists are common (a list of visit dates) but the feature is useful without them.

**Independent Test**: Load and export arrays of each tag, then try the mixed forms the ADR rejects.

**Acceptance Scenarios**:

1. **Given** an array whose elements are all tagged strings with the same recognized tag, **When** it is loaded, **Then** the property is a list of that type, and exports as the same array (ADR-011 section 2.4 and 2.6).
2. **Given** an array that mixes a recognized tag with plain strings, or two recognized tags, **When** it is loaded or checked, **Then** it fails with `value.array-mixed` at the first element that differs (ADR-011 section 2.4).
3. **Given** an array of tagged strings with an unrecognized tag, **When** it is loaded, **Then** it is a list of strings with the drops reported (ADR-011 section 2.4).
4. **Given** a database list that mixes kinds (a date and a datetime), **When** it is exported, **Then** the export fails and lists it (ADR-011 section 2.6).

---

### User Story 5 - What cannot be written is reported, not skipped (Priority: P3)

A developer exports a database that holds a vector, or a query result that holds a map. The export fails and lists each such value with its element and key; nothing is skipped or approximated.

**Why this priority**: These are existing behaviors that the ADR confirms and tests should keep (ADR-011 section 2.1 and 2.8).

**Independent Test**: Export databases holding a vector, and check the error lists it; check that no tag produces a vector on load.

**Acceptance Scenarios**:

1. **Given** a database with a vector property, **When** it is exported, **Then** the export fails with a listed error naming the element and key, and writes nothing.
2. **Given** a file that tries to write a vector as a tag (`vector`), **When** it is loaded, **Then** the tag is unrecognized: the content loads as a string and the drop is reported.

---

### User Story 6 - The documentation and the fixer say what changed (Priority: P3)

A developer reading the supported-gram page, the diagnostics contract or the changelog finds the new tags, the new codes, and the note that a file which used one of these tags as a plain label now loads typed values or fails.

**Why this priority**: The behavior change is visible to existing users; the project's documentation convention (CLAUDE.md) requires the tables to follow.

**Independent Test**: Check the docs table, the contract and the changelog against the codes the implementation emits.

**Acceptance Scenarios**:

1. **Given** the new codes, **When** the documentation is read, **Then** each appears in `docs/supported-gram.md` with its meaning and the rewrite, and in the diagnostics contract.
2. **Given** a file with a bad typed tag, **When** `fix` runs, **Then** it leaves the value unchanged and annotates the error, since no mechanical fix exists (ADR-010 section 4, ADR-011 section 4).

---

### Edge Cases

- A tag spelled in a different case (`Date`) is unrecognized, so it is dropped and reported.
- A recognized tag on an empty or whitespace-only string is a bad value.
- A recognized tag written as a fenced tagged string (the grammar's second spelling, with the tag after the opening fence) is read the same as the backtick form.
- A typed value as the value of a configured identity property is not usable as an identity, like a list: the element is treated as lacking the property and the load reports `id.property-missing`. A typed value cannot be an `@id`, since keys starting with `@` are errors.
- `--tags error` still applies to unrecognized tags only; recognized tags are always typed.
- A nanosecond fraction of more than nine digits is a bad value.
- Records inside a node's map, or a nested map as a value, remain errors (`value.map`).

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The loader MUST give each tag of ADR-011 section 2.2 its Relate type, with the content form and ranges stated there, and MUST store the value as that type.
- **FR-002**: A recognized tag with content that breaks its form or a range MUST fail with `value.tag-content` at the tagged string's position, naming the tag and the fault (ADR-011 section 2.3).
- **FR-003**: `check` MUST report every form and range error of FR-002 without a database, except whether a zone name is known and matches the offset, which the load reports (ADR-011 section 2.2).
- **FR-004**: A load that fails MUST leave the database unchanged, including when the database rejects a value the loader accepted (ADR-011 section 2.3).
- **FR-005**: Unrecognized tags MUST keep the drop-and-report behavior and the `tags` setting (ADR-006 section 2.3).
- **FR-006**: Arrays of tagged strings MUST follow ADR-011 section 2.4: one recognized tag gives a typed list, a mix gives `value.array-mixed`.
- **FR-007**: The exporter MUST write each typed value as a tagged string in the form of ADR-011 section 2.2 and 2.6, and an export of unchanged data MUST be byte-identical each time.
- **FR-008**: Export followed by load MUST give equal values and types for every typed value, including a list of one typed kind.
- **FR-009**: The exporter MUST still fail, listing every offender before writing, on a vector, a mixed-kind list, and the other values ADR-006 section 2.6 and ADR-011 section 2.6 leave unwritable.
- **FR-010**: Typed values MUST be written to the database through Relate's public interface only, building values with Cypher constructors (ADR-011 section 2.7), and the public-interface check MUST still pass.
- **FR-011**: Typed tags in the top-level record MUST be validated and not stored (ADR-011 section 2.2).
- **FR-012**: The documentation (`docs/supported-gram.md`, the diagnostics contract, the changelog and the README's list of rejected and unwritable values) MUST describe the new behavior and codes.

### Key Entities

- **Typed value**: a property value of one of the eight Relate types (date, zoned time, local time, zoned datetime, local datetime, duration, point, UUID), or a list of one of them.
- **Recognized tag**: one of the eight tag names, lower case; any other tag is unrecognized.
- **Content form**: the text grammar and range rules a tag's string must follow (ADR-011 section 2.2).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: For each of the eight types, a database value exported and reloaded has the same type and equal value, with no loss of nanoseconds, sign, dimension or zone name.
- **SC-002**: Every invalid form listed in ADR-011 section 2.2 and 2.3 has a fixture that fails with the stated code at the stated line and column, and leaves a database holding known content unchanged.
- **SC-003**: `check` on every such fixture reports the same error without a database, except the unknown-zone fixtures, which `check` accepts and the load rejects.
- **SC-004**: Loading a file of 10,000 elements in which every element carries typed values takes no more than three times as long as the same file with plain strings.
- **SC-005**: The existing corpus, round-trip, position and performance tests pass unchanged except where ADR-011 changes the expected outcome, and each such change is named in the changelog.
- **SC-006**: A file that used one of the eight tags before this change and now loads differently is documented in the changelog with the rewrite to restore the old behavior.

## Assumptions

- Relate 1.0.x constructors behave as SPIKE-005 recorded; a Relate release that changes them needs the spike's probe rerun.
- The eight tag names and content forms are exactly those of ADR-011 section 2.2; adding a tag or a form is a new decision.
- Vectors, maps, null, nodes, relationships, paths and byte arrays stay unwritable (ADR-011 section 2.1 and 2.8).
- The empty-list behavior of ADR-011 section 2.5 is already implemented and is not changed here.
- The fixer gains no new fix (ADR-011 section 4).
