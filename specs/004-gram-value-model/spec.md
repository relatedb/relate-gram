# Feature Specification: gram-value as the Value Model

**Feature Branch**: `adopt-gram-value-model`

**Created**: 2026-11-15

**Status**: Draft

**Input**: User description: "implement ADR-013: the loader holds gram_value::Value with positions, and the exporter writes each property value as gram-value's canonical text."

This specification implements ADR-013 (accepted). It names the sections of ADR-013, ADR-012, ADR-011 and ADR-006 that decide each behavior and does not restate them; where they differ, ADR-013 wins over ADR-006 for string spelling and over ADR-012 where it says so. It changes the value type inside the crate and the text the exporter writes for strings. It adds no tag, no command and no storage type.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Everything that loaded still loads, and everything rejected is still rejected (Priority: P1)

A developer has gram files that load today, and files that are rejected with a located error. After the change, the first group loads into the same database content, and the second group fails with the same code at the same position.

**Why this priority**: The loader's value type is replaced underneath every load; the strictness rule (Constitution II) and the position tests must hold (ADR-013 section 2.2).

**Independent Test**: Run every fixture in `tests/fixtures/valid` and `tests/fixtures/invalid`; compare database dumps and diagnostics with those before the change.

**Acceptance Scenarios**:

1. **Given** each valid fixture, **When** it is loaded, **Then** the stored graph equals the graph stored before the change, including integers, floats, booleans, strings, typed values and lists.
2. **Given** each invalid fixture, **When** it is loaded or checked, **Then** the code, line and column are those of its `.expect` file.
3. **Given** an integer `-0` alone, in a list, and as a record value, **When** it is loaded, **Then** it is stored as 0 (ADR-013 section 2.2).
4. **Given** an integer outside 64 bits, an octal literal, a range, a measurement, a nested map or an unsupported value, **When** it is loaded, **Then** it fails with the same code as before.
5. **Given** a decimal that is not finite, one that rounds to zero, and one that a 64-bit float cannot hold exactly, **When** they are loaded, **Then** the first two fail with `value.decimal` and the third loads as the nearest float with no loss reported (ADR-013 section 2.2).
6. **Given** an array that mixes kinds, or tags (a `date` and a `datetime`), **When** it is loaded, **Then** it fails with `value.array-mixed` at the first differing element, and the message names the tag for a tagged element (ADR-013 section 2.2).

---

### User Story 2 - Strings are written as other gram tools write them (Priority: P1)

A developer exports a database and gets `status: active` and `name: "Ada Lovelace"`: a string that is a symbol is bare, any other string is double-quoted. Another gram tool built on gram-value writes the same property values the same way. Reloading the export gives equal values.

**Why this priority**: It is the one change a user sees in the written file (ADR-013 section 2.6), and the reason the ADR exists.

**Independent Test**: Export a database with strings of each shape (symbol, with spaces, `true`, empty, digits only, non-ASCII, with escapes), check the text against gram-value's canonical text, reload, and compare values.

**Acceptance Scenarios**:

1. **Given** a string property that is a symbol (`active`, `a-b`, `foo.bar`), **When** it is exported, **Then** it is written bare.
2. **Given** a string that is `true` or `false`, is empty, starts with a digit, or has a space, a quote, a backslash or a control character, **When** it is exported, **Then** it is double-quoted with the escapes of ADR-006 section 2.5, and reloads equal.
3. **Given** an export, **When** it is loaded into an empty database, **Then** every property has the same type and value as the original (Constitution IV).
4. **Given** integers, floats, booleans, lists, points and the other typed values, **When** they are exported, **Then** they are written exactly as before the change (ADR-013 section 2.6).
5. **Given** a string with a NUL character, in a property or in a list, **When** the database is exported, **Then** the export fails and lists it, as before (ADR-013 section 2.5).
6. **Given** a nested list or a list of mixed kinds, **When** it is exported, **Then** the export fails and lists it with the element and key; an empty list is written `[]` (ADR-013 section 2.5).

---

### User Story 3 - Strings that begin with a slash round-trip (Priority: P1)

A developer has strings such as `// not a comment`, `/** x */ y` and `  /x`. They export and reload without change, and the text for a string value no longer carries a backslash before the leading slash.

**Why this priority**: ADR-013 section 2.6 removes the escape only on the condition that tree-sitter-gram 0.7.0 reads these back; this story is that condition.

**Independent Test**: A deterministic round-trip test over the listed shapes, as a string value, as a key, and as an identifier.

**Acceptance Scenarios**:

1. **Given** strings that begin with `//`, with `/**` followed later by `*/`, and with leading blanks before a slash, **When** they are exported as property values and reloaded, **Then** the reloaded values are equal and no escape of the slash appears in the text.
2. **Given** keys, labels, relationship types and identifiers that begin with a slash, **When** they are exported, **Then** they are written as before, with the escape (ADR-013 section 2.6), and reload equal.
3. **Given** the test of scenario 1 fails for any shape, **When** the work is reviewed, **Then** the escape stays for string values and the failure is reported to the user (ADR-013 section 2.6).

---

### User Story 4 - Typed values and points still load and export (Priority: P1)

A developer loads and exports the eight typed tags and typed lists. Nothing visible changes from ADR-012.

**Why this priority**: The typed value type changes (a tagged value holds its normal content), and the typed write path reads from it (ADR-013 section 2.3).

**Independent Test**: The typed fixtures and the typed contract and round-trip tests pass unchanged.

**Acceptance Scenarios**:

1. **Given** each valid typed fixture, **When** it is loaded, **Then** the stored types and values are those of ADR-011 and ADR-012.
2. **Given** a typed value the database rejects (an unknown zone, a wrong offset), **When** it is loaded, **Then** the load fails at the tagged string, or at the array element, with the same code and position as before.
3. **Given** a point, **When** it is loaded and exported, **Then** the text is `SRID=n;x y [z]` as in ADR-012 section 2.6.
4. **Given** the performance test, **When** it runs, **Then** typed loading stays within the ceiling of SC-004.

---

### User Story 5 - The change is documented and the library break is named (Priority: P2)

A developer who uses relate-gram as a library, or reads its documents, finds the changed types, the string spelling and the changed examples in the changelog, specs and contracts.

**Why this priority**: The library interface changes against 0.1.0 (ADR-013 section 4) and the documents must say so.

**Independent Test**: Search the documents for `PropValue` and for quoted-string export examples; read the changelog.

**Acceptance Scenarios**:

1. **Given** the changelog, **When** it is read, **Then** its Unreleased section lists the removed and changed public items against 0.1.0 and the bare-string export change.
2. **Given** the documents named in ADR-013 section 4, **When** they are read, **Then** none names `PropValue` as current and none shows a symbol-like string quoted in exported output.
3. **Given** ADR-006, **When** it is read, **Then** its header says ADR-013 amends section 2.5.

---

### Edge Cases

- A decimal literal with trailing zeros (`1.50`) loads as 1.5; `-0.0` loads and exports as `-0.0`.
- A string that is a symbol but looks like a number (`e5`, `a-1.5`, `x1`) is written bare and reads back as a string; one that starts with a digit is quoted.
- A string with non-ASCII text is written as itself in UTF-8.
- A key that is a symbol is written bare, and a key that is not is backtick-quoted, as before; keys are not written by the value writer.
- A record with a repeated key reports `key.duplicate` at the second key with the first as related, as before.
- A list element that fails the profile or the database reports at that element.
- A tagged string with an unrecognized tag loads as a string with the drop warning, as before.
- A typed value used as an identity property is treated as lacking the property, as before.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The loader MUST represent every property value it stores as a `gram_value::Value` of kind integer, decimal, boolean, string, tagged or array, together with the position of the value and of each array element, and `PropValue` MUST be removed (ADR-013 section 2.1).
- **FR-002**: The lowering MUST keep the diagnostic codes, messages' subjects and positions of every rejection it makes today, and MUST read `-0` as the integer 0 (ADR-013 section 2.2).
- **FR-003**: A decimal that `gram-value` reports as a big decimal MUST be loaded as the nearest finite float, with the existing non-finite and rounds-to-zero errors, and no loss reported (ADR-013 section 2.2).
- **FR-004**: The array homogeneity rule MUST compare kinds and, for tagged elements, tags, and report `value.array-mixed` at the first differing element with the tag name in the message (ADR-013 section 2.2).
- **FR-005**: A recognized tag MUST become a tagged value holding the tag and the normal content; the typed write step MUST build the Relate value from them, interpreting a point again for its numbers; `TypedValue`, `TypedContent` and `PointContent` MUST be removed (ADR-013 section 2.3).
- **FR-006**: One conversion from a lowered value to a Relate value MUST exist for integer, decimal, boolean, string and array, and MUST return an error for any other kind (ADR-013 section 2.4).
- **FR-007**: The exporter MUST write each property value as `gram_value::Value::to_text()` of the mapping of ADR-013 section 2.5, and MUST keep writing elements, identifiers, labels, types and keys itself.
- **FR-008**: The exporter MUST keep the errors for a string with NUL, a nested list, a mixed list, a non-finite float and the other unwritable values, naming the element and key, and MUST write an empty list as `[]` (ADR-013 section 2.5).
- **FR-009**: A string value MUST be written without an escape of a leading slash, after the round-trip test of User Story 3 passes; names keep it (ADR-013 section 2.6).
- **FR-010**: Export of any value other than a string MUST be byte-identical to the output before the change (ADR-013 section 2.6).
- **FR-011**: `names::string_literal`, `scalar_text`, `float_text` and `temporal_value_text` MUST be removed when nothing uses them; `names::escape_into` stays for backtick-quoted names.
- **FR-012**: The tests, fixtures and documents named in ADR-013 section 4 MUST be updated, and the changelog MUST list the changed public items against 0.1.0 (ADR-013 section 4).
- **FR-013**: Everything not named here MUST behave as before: error codes, positions, the cap on reported errors, transactional loads, the tag policy, identity rules, the fixer and the renderer.

### Key Entities

- **Lowered value**: a `gram_value::Value` of the six kinds the loader stores, with its position and, for an array, the positions of its elements.
- **Record**: an ordered list of keys and lowered values for one element, with duplicate keys found during lowering.
- **Canonical text**: gram-value's one written form of a value; the exporter's output for every property value.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every valid fixture loads to the same graph, and every invalid fixture fails with the same code, line and column, as before the change; the comparison is the existing contract, round-trip and position tests plus the fixtures, with no expectation changed except those that name quoted strings.
- **SC-002**: For each string shape of User Story 2 and User Story 3, export and reload give an equal value, in a test that does not depend on random generation.
- **SC-003**: Exports of integers, floats, booleans, lists, points and other typed values are byte-identical to the output before the change, shown by the unchanged golden strings of those types.
- **SC-004**: Loading 10,000 typed elements takes at most 1.2 times 348.7 ms, the figure measured after ADR-012 (`specs/003-gram-value-tags/tasks.md`), in a release build on the same machine.
- **SC-005**: The crate no longer defines `PropValue`, `TypedValue`, `TypedContent`, `PointContent` or `names::string_literal`, and adds no dependency (`cargo tree` is unchanged).
- **SC-006**: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `scripts/check-public-api.sh` and `cargo test` pass.

## Assumptions

- gram-value 0.1.x behaves as its conformance corpus states, as in ADR-012.
- tree-sitter-gram 0.7.0 reads strings that begin with `//` or `/**` as strings; User Story 3 proves it, and if it fails, FR-009 does not apply.
- The loader's accepted kinds are unchanged; reporting a precision-losing decimal is a separate decision (ADR-013 section 3.5).
- Typed tags are unchanged in behavior from ADR-012.
- The first release containing this change is 0.2.0, with typed tags, and is a breaking minor for library users.
