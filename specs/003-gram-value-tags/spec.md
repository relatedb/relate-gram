# Feature Specification: Known Tags from gram-value

**Feature Branch**: `adopt-gram-value`

**Created**: 2026-11-15

**Status**: Draft

**Input**: User description: "adopt gram-value for the typed tags: implement ADR-012 (validate the eight tags through gram-value's known-tag interpretation plus a Relate profile; points as `SRID=n;x y [z]`; remove the hand-written validators)."

This specification implements ADR-012 (accepted). It names the sections of ADR-012, ADR-011 and ADR-006 that decide each behavior and does not restate them; where they differ, the ADR wins, and ADR-012 wins over ADR-011 where it amends it. It changes how the typed tags of `specs/002-typed-tags/` are validated and how points are spelled. It adds no tag, no storage type and no command.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Points are written the way other gram tools write them (Priority: P1)

A developer has a gram file produced by another gram tool, with a point written ``point`SRID=4326;12.5 55.7` ``. They load it into Relate and the property is stored as a geographic point with longitude 12.5 and latitude 55.7. They export the database and get the same spelling back.

**Why this priority**: The point form is the one visible change in the accepted formats (ADR-012 section 2.4), and files from other tools only load once it holds.

**Independent Test**: Load a file with a two-dimensional and a three-dimensional point in each supported reference system, check the stored values, export, and compare the exported text with the input.

**Acceptance Scenarios**:

1. **Given** points with SRID 4326, 7203, 4979 and 9157 and the matching number of coordinates, **When** the file is loaded, **Then** each is stored as a point of that system with those coordinates (ADR-012 section 2.3).
2. **Given** a database with points, **When** it is exported, **Then** each is written as `SRID=n;x y` or `SRID=n;x y z`, with each coordinate as the shortest decimal that reads back equal (`-74`, `40.7`; `-0` as `0`), and a reload gives equal values (ADR-012 section 2.6).
3. **Given** a point in extended well-known text (``point`SRID=4326;POINT(1 2)` ``), a point with no SRID (``point`30 10` ``), an unsupported SRID, the wrong number of coordinates for the SRID, or a latitude outside [-90, 90] for 4326 and 4979, **When** it is loaded or checked, **Then** it fails with `value.tag-content` at the tagged string's position (ADR-012 sections 2.2 to 2.4).
4. **Given** a point with no SRID, **When** the error is read, **Then** it says the general tag accepts the text and Relate needs an SRID (ADR-012 section 2.3).

---

### User Story 2 - A time without seconds loads (Priority: P1)

A developer writes ``time`17:35Z` `` or ``datetime`2024-01-15T10:30+01:00` ``, as the grammar's own examples do. It loads as the same time with seconds zero. A time with no offset still fails.

**Why this priority**: It removes a rule that only concerned spelling and keeps the one rule that protects meaning (ADR-012 section 2.3).

**Independent Test**: Load ``time`17:35Z` ``, ``time`17:35:00Z` `` and ``localtime`17:35` `` and compare the stored values; check ``time`17:35` `` and a datetime with no offset are rejected.

**Acceptance Scenarios**:

1. **Given** a `time`, `datetime`, `localtime` or `localdatetime` without seconds, **When** it is loaded, **Then** it equals the value with seconds zero (ADR-012 section 2.3).
2. **Given** a `time` or `datetime` with no offset, **When** it is loaded or checked, **Then** it fails with `value.tag-content` and a message that says the offset is required (ADR-012 section 2.3).
3. **Given** a time with an offset written `+00:00`, a fraction with trailing zeros, or a UUID in upper case, **When** it is loaded, **Then** the stored value equals the one the normal form denotes (ADR-012 section 2.2).

---

### User Story 3 - Bad values are rejected as before, with gram-value's message (Priority: P1)

A developer writes a date with month 13, a duration in weeks, a UUID without hyphens or any other bad value that the typed tags rejected before. The load still fails, leaves the database unchanged and reports `value.tag-content` at the same position; the message is now gram-value's.

**Why this priority**: The strictness rule (Constitution II) must hold through the change; nothing a user was told is bad may start loading, except what ADR-012 names.

**Independent Test**: Run every `tests/fixtures/invalid/tag-content-*` fixture; each fails with the same code, line and column, except the ones this specification lists as changed.

**Acceptance Scenarios**:

1. **Given** each invalid typed-tag fixture that remains invalid, **When** it is loaded or checked, **Then** the code and position are those of the fixture's expectation (ADR-012 section 2.2).
2. **Given** a failed load, **When** the database is inspected, **Then** it is unchanged (ADR-011 section 2.3).
3. **Given** a `datetime` with a zone name, **When** it is checked, **Then** `check` accepts the zone name, and a load with an unknown zone or a wrong offset fails as before (ADR-012 section 2.3, ADR-011 section 2.2).
4. **Given** the error for bad content, **When** its message is read, **Then** it names the tag once and does not repeat gram-value's own tag prefix (ADR-012 section 2.2).

---

### User Story 4 - The tags gram-value knows and Relate lacks stay unrecognized (Priority: P2)

A developer writes ``week`2024-W05` `` or ``weekDate`2024-W05-1` ``. Relate has no type for either, so they are handled like any unrecognized tag: stored as strings, with the drop reported.

**Why this priority**: The set of known tags in gram-value is larger than Relate's; the boundary must not move silently (ADR-012 section 2.5).

**Independent Test**: Load a file with each tag and check the stored string, the warning, and the `--tags error` behavior.

**Acceptance Scenarios**:

1. **Given** a `week` or `weekDate` tagged string, **When** it is loaded with the default tag policy, **Then** it is stored as a string and the drop is reported (ADR-006 section 2.3).
2. **Given** the same file with `--tags error`, **When** it is loaded, **Then** it fails like any unrecognized tag.

---

### User Story 5 - The change is documented and nothing else moves (Priority: P2)

A developer reading the supported-gram page, the quickstart and the changelog finds the new point form, the dropped seconds rule and the migration note for files that used extended well-known text. The rest of relate-gram behaves as before.

**Why this priority**: Typed tags are unreleased, so the documents must describe the state a first release will have (ADR-012 section 4).

**Independent Test**: Read the docs against the behavior of the fixtures; run the full test suite.

**Acceptance Scenarios**:

1. **Given** the supported-gram page and the changelog, **When** they are read, **Then** they show the point form of ADR-012 section 2.4 and no extended well-known text, and the changelog's Unreleased entry says that a file with the old spelling needs rewriting.
2. **Given** the full test suite, **When** it runs, **Then** every test passes, and every test or fixture whose expectation changed is one named in ADR-012 section 4.
3. **Given** a file with a bad typed tag, **When** `fix` runs, **Then** it leaves the value and annotates the error, with no fix for the old point spelling (ADR-012 section 2.4, ADR-010 section 4).

---

### User Story 6 - The old and new validators are shown to agree (Priority: P2)

A maintainer wants to know that replacing about 700 lines of validators changed nothing but what the ADR says. Before the old validators go, both run over every typed fixture and the accepted and rejected tables of `specs/002-typed-tags/`, and the cases where they differ are listed.

**Why this priority**: It is the evidence that the replacement is safe (ADR-012 section 2.7), and the list of differences becomes part of this specification.

**Independent Test**: Run the comparison and check that every difference is in the list below, and that the list has no entry the comparison did not find.

**Acceptance Scenarios**:

1. **Given** the typed fixtures and the tables, **When** both validators run, **Then** the accept or reject outcome differs only in the expected differences section.
2. **Given** the comparison finished, **When** the old validators are deleted, **Then** the comparison's inputs remain as fixtures and the list of differences is recorded here.

---

### Edge Cases

- A point coordinate in exponent form (`1e3`) is rejected by gram-value and stays rejected; the message is gram-value's.
- A `-0` coordinate is stored as written and exported as `0`.
- A point with NaN or an infinite coordinate is rejected.
- A fenced tagged string with a known tag is read like the backtick form; the profile applies to it equally.
- A tag in a different case (`Point`, `Date`) is unrecognized and dropped with a report.
- A `time` or `datetime` with a zone name instead of an offset is rejected by the offset rule; a `datetime` with an offset and a zone name still loads.
- A list of points or times follows ADR-011 section 2.4 with the new forms; a list element that fails the profile reports at that element.
- Typed tags in the top-level record are validated through the same path and not stored.
- A string that gram-value would read as a known tag but Relate does not store (`week`, `weekDate`) is not a bad value; it is an unrecognized tag.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Validation of each of the eight recognized tags MUST call gram-value's interpretation of the tag and content, and a diagnostic from it MUST become a `value.tag-content` error at the tagged string's position with gram-value's message, without its tag prefix (ADR-012 section 2.2).
- **FR-002**: After a successful interpretation the Relate profile MUST be applied: `time` and `datetime` require an offset; `point` requires SRID 4326 or 7203 with two coordinates, or 4979 or 9157 with three, and a latitude in [-90, 90] for 4326 and 4979. A profile failure MUST be a `value.tag-content` error whose message says the general tag accepts the text and Relate does not (ADR-012 section 2.3).
- **FR-003**: Seconds MUST NOT be required on `time`, `localtime`, `datetime` or `localdatetime` (ADR-012 section 2.3).
- **FR-004**: The loader MUST pass the normal content string of the interpretation to Relate for the text tags, keep a zone name as written, and build a point from the numbers of gram-value's point (ADR-012 section 2.2).
- **FR-005**: A point MUST be accepted only as `SRID=n;x y [z]`; extended well-known text and a point with no SRID MUST be rejected (ADR-012 section 2.4).
- **FR-006**: The exporter MUST write a point as `SRID=n;x y [z]` with each coordinate as the shortest decimal that reads back equal, `-0` as `0`; plain float properties MUST keep their `.0`; every other typed value MUST be written as before (ADR-012 section 2.6).
- **FR-007**: `week` and `weekDate` MUST remain unrecognized and follow the tag policy (ADR-012 section 2.5).
- **FR-008**: The hand-written validators for dates, times, durations, UUIDs and points in `src/typed/` MUST be removed once the comparison of FR-009 is recorded; the `typed` module keeps `Tag::from_name`, `Tag::ALL`, the profile and the content types the loader needs.
- **FR-009**: Before removal, the old and new validators MUST be compared over the typed fixtures and the accepted and rejected tables of `specs/002-typed-tags/`, and every difference MUST be listed in the expected differences section with its reason (ADR-012 section 2.7).
- **FR-010**: The dependency MUST be `gram-value = "0.1"`, MUST add no other runtime dependency, and the public-interface check MUST still pass (ADR-012 section 2.1, Constitution V).
- **FR-011**: Tests, fixtures and documents named in ADR-012 section 4 MUST be updated; the changelog's Unreleased section MUST state the point form and that files with the old spelling need rewriting; ADR-011's amendment MUST stay consistent with the behavior.
- **FR-012**: Everything not named here MUST behave as before: error codes, positions, the cap on reported errors, transactional loads, the tag policy, identity rules and the fixer.

### Key Entities

- **Known tag**: a tag name that gram-value interprets. Relate-gram recognizes the eight of them that Relate stores; `week` and `weekDate` are known to gram-value and unrecognized here.
- **Interpretation**: gram-value's result for a tag and content: the value's parts, its normal content string, or diagnostics.
- **Profile**: the narrowing Relate's types need beyond the general form (offset required, supported SRIDs and dimensions, latitude range).
- **Normal content**: the one spelling gram-value gives to a value, which the loader passes to Relate.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every `tests/fixtures/invalid/tag-content-*` fixture either still fails with the same code, line and column, or is listed in the expected differences with the reason; none is silently dropped.
- **SC-002**: For each of the eight types, a database value exported and reloaded has the same type and equal value, as in `specs/002-typed-tags/` SC-001, with points in the new form.
- **SC-003**: A file produced by another gram tool with points in the general form, with SRIDs in the profile, loads without change.
- **SC-004**: Loading 10,000 elements that all carry typed values takes no more than 1.2 times as long as before this change on the same machine.
- **SC-005**: The number of lines in `src/typed/` falls by at least 600, and the crate's runtime dependencies grow by exactly one (`gram-value`).
- **SC-006**: The corpus, round-trip, position, contract and performance tests pass, with each changed expectation named in ADR-012 section 4.

## Expected Differences

Recorded by the comparison of FR-009 (task T010): the old validators and `Tag::validate` ran over 238 string literals (every literal in the old validators' unit tests and every typed fixture) under each of the eight tags; 66 were accepted by at least one. These are all the differences; each is explained by ADR-012.

| Input | Before | After | Reason |
|---|---|---|---|
| ``time`10:30Z` ``, ``localtime`10:30` ``, ``datetime`2024-01-15T10:30Z` ``, ``localdatetime`2024-01-15T10:30` `` | rejected | loads | Seconds are spelling (ADR-012 section 2.3). |
| ``point`SRID=4326;POINT(-74.0 40.7)` `` and the other nine extended well-known text points (SRIDs 4326, 7203, 4979, 9157; latitude -90 and 90) | loads | rejected | The point form is gram-value's (ADR-012 section 2.4). |
| ``point`SRID=4326;1 2` `` and its kind (not among the old tests' literals; known from the forms) | rejected | loads | Same. |
| ``point`30 10` `` | rejected (form) | rejected (profile) | Same outcome; the message says Relate needs an SRID (ADR-012 section 2.3). |

No other input changed outcome. Differences in value, not outcome (an offset `+00:00`, a fraction `.50`, a coordinate `30.0`, an upper-case UUID) are covered by unit tests of `Tag::validate`: the loader passes the normal form, which denotes the same value (ADR-012 section 2.2).

## Assumptions

- gram-value 0.1.x behaves as its conformance corpus states; a minor version that changes a form needs a deliberate bump and a look at the profile (ADR-012 section 4).
- Relate 1.0.x constructors behave as SPIKE-005 recorded.
- `PropValue` and the lowering of tree-sitter nodes stay; adopting `gram_value::Value` is a later ADR (ADR-012 section 3.4).
- Typed tags have not been released, so no released behavior changes; the first release that contains them is 0.2.0.
- The fixer gains no new fix.
