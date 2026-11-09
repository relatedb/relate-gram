# Feature Specification: Phase 1 Gram Load and Export

**Feature Branch**: `001-phase1-load-export`

**Created**: 2026-11-09

**Status**: Draft

**Input**: User description: "phase 1"

This specification implements the first phase of RFC-003 (§5.9): one unconstrained graph space, with no named graphs, no ontology and no metadata nodes. It names the accepted ADRs that decide each behavior and does not restate them; where they differ, the ADR wins (Constitution I). The ADRs are ADR-001 (load identity and merge), ADR-002 (export identity), ADR-003 (file kind and the top-level record), ADR-004 (preserved identity), ADR-006 (property values) and ADR-007 (parsing).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Load a gram file into an in-memory database (Priority: P1)

A developer has a gram file that describes a small property graph, for example people and the relationships between them. They hand the file's text to the library and get a Relate in-memory database holding exactly that graph, which they can then query with Cypher.

**Why this priority**: Loading is the reason the crate exists. Without it there is nothing to export, reload or query.

**Independent Test**: Load a file of nodes, relationships and a path into an empty database, then query node counts, labels, relationship types, endpoints and property values, and compare them with what the file says.

**Acceptance Scenarios**:

1. **Given** a file with labelled nodes with properties and typed relationships between them, **When** it is loaded into an empty database, **Then** the database holds those nodes and relationships with their labels, types, endpoints and properties, in the order the file introduced them.
2. **Given** a file that writes a path such as `(a)-->(b)-->(c)`, **When** it is loaded, **Then** it yields the same elements as writing each relationship separately (ADR-001 §2.6).
3. **Given** a file whose elements carry identifiers, **When** it is loaded, **Then** each element carries its identifier as the reserved property `@id`, and an element without an identifier carries none, with the load reporting how many such elements it created (ADR-004; ADR-001 §2.1, §2.5).
4. **Given** a file whose properties use each accepted value kind (strings in every quoting style, integers, hexadecimal integers, decimals, booleans, arrays of one scalar kind), **When** it is loaded, **Then** each value is stored as the Cypher value ADR-006 §2.1 and §2.2 give.
5. **Given** a file with comments, **When** it is loaded, **Then** the comments have no effect on the result and nothing is reported about them (ADR-007 §2.5).

---

### User Story 2 - Reject what does not map to Cypher, with a position (Priority: P1)

A developer loads a file that contains something the first phase does not support, or that contradicts itself. The load fails, leaves the database unchanged, and says what was wrong and where in the file, so they can fix the file.

**Why this priority**: The strictness rule is the product's central promise (Constitution II). A loader that quietly approximates is worse than no loader.

**Independent Test**: Load a set of invalid files, one per rejected construct, into a database holding known content, and check that every load fails with the construct named and the line and column of the offending text, and that the database is unchanged.

**Acceptance Scenarios**:

1. **Given** a file using a range, a measurement, a nested map, an octal literal, a mixed-type array, an integer beyond 64 bits, or an annotation, **When** it is loaded, **Then** it fails with the construct named and its position (ADR-006 §2.1, §2.2; ADR-007 §2.4).
2. **Given** a file that defines one element twice, uses one identifier as both a node and a relationship, repeats a property key, or uses a key starting with `@`, **When** it is loaded, **Then** it fails and reports the positions involved (ADR-001 §2.2 to §2.4, §2.8; ADR-006 §2.4).
3. **Given** a file with malformed syntax, **When** it is loaded, **Then** every syntax error found is reported with its position, up to a cap, with the total (ADR-007 §2.2).
4. **Given** a file whose top-level record has a `kind` other than `property-graph`, **When** it is loaded, **Then** it fails with a message stating the supported kind (ADR-003 §2.1).
5. **Given** a file containing tagged strings, **When** it is loaded, **Then** each tag is dropped, the content is stored as a string, and every drop is reported with its position, or the load fails if the caller asked for tags to be errors (ADR-006 §2.3).
6. **Given** a file with a top-level record, **When** it is loaded, **Then** the load succeeds, the graph is unaffected, and a warning lists the record's properties as not stored (ADR-003 §2.2).

---

### User Story 3 - Reload an edited file into the same database (Priority: P2)

A developer loads a file, edits it, and loads it again into the same database. Elements the file identifies are matched to the ones already stored, instead of being created again.

**Why this priority**: Without it, every reload duplicates the graph (ADR-004's reason for preserving identity). It needs Story 1.

**Independent Test**: Load a file, change one property value and add one element, load it again, and check the element count grew by one and the changed value is the file's.

**Acceptance Scenarios**:

1. **Given** a database loaded from a file, **When** the same file is loaded again, **Then** the database is unchanged (ADR-001 §2.7).
2. **Given** an edited file with a changed property, an added label and a new element, **When** it is loaded over the earlier load, **Then** the matched element takes the file's property values and gains the label, properties the file does not state stay, and the new element is created (ADR-001 §2.7).
3. **Given** a reload where a later error is found while matching (for example a relationship whose type differs from the stored one), **When** the load fails, **Then** nothing from that load is written (ADR-001 §2.7).
4. **Given** a caller that names an identity property for nodes or relationships, **When** a file is loaded, **Then** elements are matched by that property's value, and no `@id` is written for that kind (ADR-001 §2.10).

---

### User Story 4 - Export a database to a gram file (Priority: P2)

A developer takes a Relate database, which may have been built by Cypher rather than loaded from gram, and writes it out as a gram file they can read, diff and edit.

**Why this priority**: It completes the interchange loop (RFC-001), and it is independently useful for any database.

**Independent Test**: Build a database with Cypher, export it twice, and check that the two outputs are identical and that loading the output reproduces the database's graph.

**Acceptance Scenarios**:

1. **Given** a database of nodes and relationships, **When** it is exported with default settings, **Then** each element is written with an identifier chosen by ADR-002 §2.1 (its `@id` if it has one, otherwise a configured property, otherwise a generated `n0…` or `r0…`), nodes first, then relationships, in the deterministic order of ADR-002 §2.4.
2. **Given** an unchanged database, **When** it is exported twice, **Then** the two outputs are byte-for-byte equal.
3. **Given** a configured identity property that some elements lack or that is not unique, **When** the database is exported, **Then** the export fails before producing any output and lists every offending element and why (ADR-002 §2.2, §2.7).
4. **Given** a database holding a value that has no gram form (a date, a point, a non-finite float, an empty list, a key starting with `@` other than `@id`), **When** it is exported, **Then** the export fails before any output and lists every offender (ADR-006 §2.6; ADR-002 §2.7).
5. **Given** a database with values of every exportable kind, **When** it is exported, **Then** each value is written in the spelling ADR-006 §2.5 gives, and each identifier as ADR-002 §2.6 gives.

---

### User Story 5 - Round-trip a graph through gram (Priority: P3)

A developer loads a file and exports the result, or exports a database and loads the output, and gets the same graph back.

**Why this priority**: It states the project's fidelity promise (Constitution IV) as one checkable property across Stories 1 and 4.

**Independent Test**: For a set of fixture graphs, load, export and load again into a fresh database, and compare the two databases element for element.

**Acceptance Scenarios**:

1. **Given** a file the loader accepts, **When** it is loaded, exported and the output loaded into a fresh database, **Then** the two databases hold the same graph, with spellings normalized as ADR-006 §4 lists.
2. **Given** a database exported with a configured identity property, **When** the output is loaded with the same setting, **Then** it matches the same elements instead of duplicating them (ADR-002 §2.1).

---

### Edge Cases

- A file with no elements, only a record, or an empty file.
- An identifier that is only referenced and never defined creates an empty node and is listed in a warning (ADR-001 §2.3).
- A relationship identifier that is only referenced has no type and is an error (ADR-001 §2.4).
- The identifiers `42` and `` `42` `` are one identifier, and the integer 42 is stored as the string `"42"` (ADR-001 §2.1).
- Input that is not valid UTF-8, a leading byte order mark, and CRLF line endings (ADR-007 §2.2).
- An empty string value, a string with newlines in a fenced string, and a property key with spaces.
- A database containing several elements with the same `@id`, which makes a load fail with a list (ADR-001 §2.7).
- A database element with no `@id`, which a load never matches (ADR-001 §2.7).
- Loading into a database that already holds data created by Cypher.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The library MUST load gram text into a Relate in-memory database and MUST export a Relate database as gram text, each as a single operation that either completes or changes nothing (ADR-001 §2.7; ADR-002 §2.7).
- **FR-002**: The loader MUST read the gram text through the tree-sitter-gram syntax tree and MUST report every syntax error with a line, a column and a byte offset (ADR-007).
- **FR-003**: The loader MUST accept nodes, relationships, paths and the top-level record, and MUST reject every other construct with an error that names the construct and its position (RFC-001 §5.1; ADR-007 §2.4).
- **FR-004**: The loader MUST map property values by ADR-006 §2.1 to §2.4, and MUST report every dropped tag (ADR-006 §2.3).
- **FR-005**: The loader MUST resolve identifiers, definitions and references, and MUST store each identifier as `@id`, by ADR-001 §2.1 to §2.8, and MUST report the number of elements it created without an identifier (ADR-004).
- **FR-006**: The loader MUST match the file's identified elements against the database by `@id`, or by a caller-named property, by ADR-001 §2.7 and §2.10.
- **FR-007**: The loader MUST check the file kind and the top-level record, and MUST report the record's properties as not stored, by ADR-003.
- **FR-008**: The loader MUST ignore comments (ADR-007 §2.5).
- **FR-009**: The exporter MUST choose identifiers by ADR-002 §2.1 to §2.3, in the order and layout of ADR-002 §2.4 and §2.5, and MUST write values by ADR-006 §2.5.
- **FR-010**: The exporter MUST fail before producing any output when an identity source is missing or not unique, or a value has no gram form, and MUST list every offending element (ADR-002 §2.2, §2.7; ADR-006 §2.6).
- **FR-011**: Two exports of one unchanged database MUST be identical, and a load followed by an export MUST be stable, as ADR-002 §2.4 requires.
- **FR-012**: The library MUST take its settings (identity properties for load and export, the export identity mode, and the tag policy) as ordinary inputs and MUST NOT depend on the `relate` command-line tool.
- **FR-013**: The library MUST depend only on Relate's public library interface (Constitution V).
- **FR-014**: Every error that concerns the source text MUST carry the position of the offending text, and a test MUST assert that position (Constitution VI).

### Key Entities

- **Gram file**: UTF-8 text holding an optional top-level record and any number of nodes, relationships and paths, each possibly identified.
- **Element**: a node or a relationship in the database, with labels or a type, properties and, when loaded from an identified gram element, an `@id`.
- **Identifier**: the text a gram file uses to name an element; file-scoped on load, and stored as `@id`.
- **Load report**: what a load tells its caller beyond success: warnings (dropped tags, unstored record properties, referenced-only identifiers), the count of elements created without an identifier, and, on failure, the located errors.
- **Settings**: the caller's choices for identity properties, export identity mode and tag policy.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every input in the tree-sitter-gram corpus that the first phase accepts loads into the graph the corpus describes, and every corpus input it excludes is rejected with an error at the expected position (ADR-007 §2.6).
- **SC-002**: For every rejected construct in User Story 2, a test shows the load fails, the reported line and column are those of the offending text, and the database is unchanged afterwards.
- **SC-003**: Loading a fixture file twice leaves the database identical to loading it once, in every fixture.
- **SC-004**: Exporting an unchanged database twice gives byte-identical output in every fixture.
- **SC-005**: Load, export and load again into a fresh database gives the same graph in every fixture, up to the normalized spellings of ADR-006 §4.
- **SC-006**: A file of 10,000 elements loads, and a database of the same size exports, in under 10 seconds on a developer laptop.
- **SC-007**: A developer new to the project can load a sample file and query it using only the library's documentation, in under ten minutes.

## Assumptions

- Only gram kind `property-graph` is supported (ADR-003); files may be named `graph-name.kind.gram`, and the name has no effect in this phase.
- The target is a Relate in-memory database that the loader writes to through Relate's public library interface; loading into a persistent store, or concurrent writers, are not tested here (SPIKE-001 left both open).
- This phase has no named graphs, no ontology and no metadata nodes (RFC-003 §5.9), so a load into a non-empty database merges by `@id` without regard to graphs.
- The command-line integration (verbs, flags, defaults) is out of scope and is decided separately; the settings are library inputs until then (ADR-006 and ADR-001 name the settings).
- Language bindings for TypeScript and Python are out of scope (RFC-001 §4.6).
- Dates, points and other non-basic Cypher values cannot be exported until a tag vocabulary is decided (ADR-006 §2.6).
- The performance figure in SC-006 is a first target; SPIKE-001 measured loads only up to about 50,000 nodes, so the figure may be revised when a plan measures it.
- The spec depends on the `relate` crate (0.16.1 at the time of the spikes) and on `tree-sitter-gram` 0.6.1.
