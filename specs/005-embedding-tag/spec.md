# Feature Specification: The embedding Tag

**Feature Branch**: `embedding-tag`

**Created**: 2026-11-15

**Status**: Draft

**Input**: User description: "implement ADR-014: the embedding tag carries Relate vectors. A ninth known tag `embedding` loads as a Relate vector, float vectors export as `embedding` tagged values in gram-value's normal form, integer vectors stay export errors."

This specification implements ADR-014 (accepted). It names the sections of ADR-014, ADR-011, ADR-012 and ADR-013 that decide each behavior and does not restate them; where they differ, ADR-014 wins over ADR-011 sections 2.8 and 3.6 for float vectors. It adds one tag and no command, setting or storage type, and it needs `gram-value` 0.2 (PR 10).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - A file with embeddings loads them as vectors (Priority: P1)

A developer has a gram file whose nodes carry an `embedding` tagged string, such as one exported by another gram tool. They load it and get a Relate vector property of the matching kind, with the same dimension and the same numbers.

**Why this priority**: Loading is the half that did not exist: today the tag is dropped with a warning and the vector is lost (ADR-014 section 1).

**Independent Test**: Load fixtures with an `F32` and an `F64` embedding of a few dimensions, read the properties back, and compare kind, dimension and every element with the bytes in the file.

**Acceptance Scenarios**:

1. **Given** an `F32` embedding, **When** it is loaded, **Then** the property is a `FLOAT32` vector of the stated dimension whose elements equal the payload's (ADR-014 section 2.2).
2. **Given** an `F64` embedding, **When** it is loaded, **Then** the property is a `FLOAT64` vector with the same elements.
3. **Given** the same embedding as a node property and as a relationship property, **When** both are loaded, **Then** both are vectors.
4. **Given** content from the reference SafeTensors writer, and content with another key order, whitespace or padding, **When** each is loaded, **Then** both give the same vector (content is read through gram-value's interpretation, ADR-014 section 2.1).
5. **Given** a file loaded before this change with an `embedding` string and the tag policy `warn`, **When** it is loaded now, **Then** the value is a vector and no `value.tag-dropped` warning is reported for `embedding`.
6. **Given** other unrecognized tags (`md`, `Date`), **When** they are loaded, **Then** they are dropped and reported as before.

---

### User Story 2 - A database with float vectors exports and round-trips (Priority: P1)

A developer has a database with embedding properties. They export it, get `embedding` tagged values, and reloading the export gives vectors with the same kind, dimension and numbers. An integer vector is refused with a message that says why.

**Why this priority**: Export previously failed for any database with a vector (ADR-014 section 1), which is the reason for the ADR.

**Independent Test**: Create vectors of each float kind, export, compare the text with gram-value's normal form, reload, and compare values; export an integer vector and read the error.

**Acceptance Scenarios**:

1. **Given** a `FLOAT32` vector and a `FLOAT64` vector, **When** the database is exported, **Then** each is written as `embedding` with gram-value's normal content for its dtype and bytes (ADR-014 section 2.3).
2. **Given** the export, **When** it is loaded into an empty database, **Then** every vector has the same kind, dimension and elements as the original (Constitution IV).
3. **Given** a vector with negative zero, subnormal and extreme finite elements, **When** it is exported and reloaded, **Then** the bytes are unchanged.
4. **Given** an `INTEGER8` to `INTEGER64` vector, **When** the database is exported, **Then** the export fails with `export.value` naming the element and key and saying that `embedding` carries float vectors only (ADR-014 section 2.3).
5. **Given** a database with float vectors and integer vectors, **When** it is exported, **Then** every integer vector is listed, and nothing is written.
6. **Given** content that was not in normal form when loaded, **When** the database is exported, **Then** the text is the normal form with the same payload bytes.
7. **Given** other typed values, strings, numbers and lists, **When** they are exported, **Then** the text is as before this change.

---

### User Story 3 - A bad or unstorable embedding is a located error, and `check` finds it without a database (Priority: P1)

A developer runs `check` on a file with a malformed embedding, a dimension above Relate's limit, a NaN, or an array of embeddings. Each is reported with a code, line and column, and the message says what is wrong and whose limit it is.

**Why this priority**: The strictness rule (Constitution II) and the position tests apply to the new tag, and some of its limits are Relate's, not the tag's (ADR-014 section 2.2).

**Independent Test**: `check` on one invalid fixture per rule; each has an `.expect` file with the code, line and column; `load` of the same file gives the same.

**Acceptance Scenarios**:

1. **Given** content that gram-value rejects (bad Base64, a malformed header, a wrong dtype such as `I8` or `f32`, a length mismatch), **When** it is checked, **Then** it fails with `value.tag-content` at the tagged string, carrying gram-value's message (ADR-014 section 2.1).
2. **Given** an embedding of dimension 4097, **When** it is checked, **Then** it fails with `value.tag-content` stating Relate's limit of 4096 and that the tag allows more (ADR-014 section 2.2); dimension 4096 passes.
3. **Given** an embedding with a NaN or an infinity, **When** it is checked, **Then** it fails with `value.tag-content` naming the index of the first non-finite element; a dimension above 4096 is reported first when both apply.
4. **Given** an array whose elements are `embedding` values, **When** it is checked, **Then** it fails with `value.tag-content` at the array, once, with the message of ADR-014 section 2.2.
5. **Given** an array that mixes an `embedding` with another value, **When** it is checked, **Then** it fails with `value.array-mixed` as before.
6. **Given** any rejected embedding in a file, **When** the file is loaded, **Then** nothing is stored (ADR-014 section 2.2; the single transaction of ADR-006).
7. **Given** `fix` on a file with a rejected embedding, **When** it runs, **Then** the file is not changed and the error is annotated (ADR-014 section 2.5).

---

### User Story 4 - 16-bit embeddings load only when Relate allows them, and the failure is reported once, before any write (Priority: P2)

A developer has `F16` and `BF16` embeddings. With Relate's preview flag on they load as `FLOAT16` and `BFLOAT16` vectors and export back byte for byte. With the flag off, the load stops before writing anything and says which values are affected and how to turn the flag on.

**Why this priority**: The two kinds are behind a preview flag in Relate (ADR-014 section 1), so they are real but conditional, and the failure must be understandable.

**Independent Test**: Run the loading tests in a process with `RELATE_PREVIEW=float16-coordinate-types` and in one without; run the exhaustive 16-bit conversion test with the flag on.

**Acceptance Scenarios**:

1. **Given** the flag on and an `F16` or `BF16` embedding, **When** it is loaded and exported, **Then** the vector has the matching kind and the export is the original normal content.
2. **Given** the flag off and a file with two `F16` embeddings and one `BF16` embedding, **When** it is loaded, **Then** the load fails with nothing written and one `value.tag-content` error at each of the three values, each naming the flag and its environment variable (ADR-014 section 2.2).
3. **Given** the flag off and a file with only `F32` and `F64` embeddings, **When** it is loaded, **Then** it loads, and no probe is run.
4. **Given** the flag off, **When** the same file is checked, **Then** `check` passes, because it cannot know the flag (ADR-014 section 4).
5. **Given** every finite 16-bit bit pattern of `F16` and `BF16`, subnormals and negative zero included, **When** the bytes are widened, stored in a Relate vector and read and narrowed back, **Then** the bytes are identical (ADR-014 section 2.4).
6. **Given** the loader, **When** it runs, **Then** it never sets the flag (ADR-014 section 2.2).

---

### User Story 5 - The change is documented (Priority: P2)

A developer reading the user documents, the changelog and the earlier decisions finds nine tags, the vector rules and the amended passages.

**Why this priority**: ADR-014 section 2.6 lists the documents that change, and several say today that vectors cannot be written.

**Independent Test**: Search the documents for "Vectors cannot be written", for `vector` as an example of an unrecognized tag, and for "eight"; read the changelog.

**Acceptance Scenarios**:

1. **Given** `docs/supported-gram.md`, **When** it is read, **Then** it lists nine tags, shows the `embedding` form, lists the four rejections of ADR-014 section 2.2 in its rejected table, and says that `check` cannot know the preview flag.
2. **Given** `CLAUDE.md`, ADR-011, ADR-010 and RFC-002, **When** they are read, **Then** each carries the amendment of ADR-014 section 2.6, with the ADR-010 fix table saying there is no fix.
3. **Given** the changelog, **When** it is read, **Then** its Unreleased section names the tag, the loading change for files that used `embedding`, and the 16-bit condition.

---

### Edge Cases

- A dimension of 1 loads; a dimension of 0 is rejected by gram-value (`out-of-range`) and reported as `value.tag-content`.
- Content with a padding or pad-bit fault, or empty content, is `value.tag-content` with gram-value's message.
- An embedding as a record property of the file's top-level record is reported as not stored (ADR-003) and not loaded.
- Identity properties: an embedding used as an identity property is treated as lacking the property, like other typed values.
- A vector inside a list property cannot be created, and an export never produces one (ADR-014 section 2.3).
- A relationship property that is an embedding loads and exports like a node property.
- With the tag policy `error`, an unrecognized tag is an error as before; `embedding` is recognized and is not affected.
- An `F32` payload with a NaN is rejected at check time even though gram-value accepts it.
- A load of many embeddings with the flag off and no 16-bit values does not run the probe.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: `embedding` MUST be a known tag in the library's tag list, lower case and matched exactly, and its content MUST be validated with `gram-value`'s interpretation, shown as `value.tag-content`, with no new diagnostic code (ADR-014 section 2.1).
- **FR-002**: A recognized `embedding` MUST be stored as a Relate vector of the matching kind, built from the payload's elements widened to f64, with the dimension equal to the payload's (ADR-014 section 2.2).
- **FR-003**: The statement that builds a vector MUST write the kind as a literal keyword, be issued per element kind, key and dtype within a batch, and run in the load's single transaction (ADR-014 section 2.2).
- **FR-004**: A dimension above 4096 and then a non-finite element MUST be `value.tag-content` errors, found by `check` without a database, with the messages of ADR-014 section 2.2.
- **FR-005**: An array whose elements are `embedding` values MUST be a single `value.tag-content` error at the array, decided before its elements' content is checked; `value.array-mixed` MUST keep its meaning (ADR-014 section 2.2).
- **FR-006**: When a load holds an `F16` or `BF16` embedding, the loader MUST run one probe before writing anything and, if it fails, stop with nothing written and one `value.tag-content` error at every such value naming the flag, its environment variable and the database's message; it MUST NOT set the flag, and it MUST NOT run the probe when no 16-bit embedding is present (ADR-014 section 2.2).
- **FR-007**: The exporter MUST write a float vector as an `embedding` tagged value in `gram-value`'s normal form, narrowing each element to the vector's kind and writing little-endian bytes (ADR-014 section 2.3).
- **FR-008**: The exporter MUST report a vector of an integer kind as `export.value` naming the element and key and saying the tag carries float vectors only, and MUST list every such vector (ADR-014 section 2.3).
- **FR-009**: The conversion of 16-bit elements MUST be code in this crate with no new dependency, and MUST be tested over every finite bit pattern of both formats (ADR-014 section 2.4).
- **FR-010**: `fix` MUST offer no correction for a rejected `embedding`, and no fix MUST remove the tag (ADR-014 section 2.5).
- **FR-011**: The documents and decisions named in ADR-014 section 2.6 MUST be amended, and the changelog MUST describe the change (ADR-014 section 2.6).
- **FR-012**: Everything not named here MUST behave as before: the eight existing tags, error codes and positions, the cap on reported errors, transactional loads, the tag policy, identity rules, the renderer, and the export of every value other than a float vector.

### Key Entities

- **Embedding value**: a `gram_value` embedding (dtype, dimension and payload bytes) carried by the tag, with the position of its tagged string.
- **Vector property**: Relate's stored form, a kind (`FLOAT16`, `BFLOAT16`, `FLOAT32`, `FLOAT64` here) and floats rounded to it.
- **Probe**: one statement run before the load writes anything, to find out whether Relate accepts 16-bit vectors in this process.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: For each of `F32` and `F64`, a file with embeddings of dimensions 1, 3, 384 and 4096 loads, exports and reloads to equal vectors, and the export of content from the reference writer is byte-identical to its input.
- **SC-002**: Every rejected shape of User Story 3 has an invalid fixture whose `check` and `load` give the `.expect` code, line and column, and no load stores anything.
- **SC-003**: With the preview flag on, the exhaustive test covers all 65,536 bit patterns of each 16-bit format and the round trip is exact for every finite pattern.
- **SC-004**: A flag-off load of a file with 16-bit embeddings stops before any write: the database is unchanged and each affected value has one error.
- **SC-005**: Existing fixtures and golden strings are unchanged except those that used `embedding` or `vector` as an example of an unrecognized tag, and loading 10,000 typed elements stays within the ceiling of the previous release's SC-004.
- **SC-006**: Loading and exporting 1,000 vectors of dimension 1,536 is measured in a release build when the feature is first complete and recorded in `tasks.md`; later changes stay within 1.2 times that figure on the same machine.
- **SC-007**: The crate adds no dependency beyond `gram-value` 0.2 (`cargo tree` differs only by that bump), and `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `scripts/check-public-api.sh` and `cargo test` pass.

## Assumptions

- `gram-value` 0.2 behaves as its ADR-008 and conformance corpus state, and its `Embedding` type is available as in the spike.
- `relate` 1.0.x behaves as the spike and its source check found: the kinds, the preview gate on a literal kind, the finite-number rule, and the dimension range of 1 to 4096.
- The preview flag is a process environment variable, so tests that need it run in a process started with it or set it before the first connection.
- Integer vectors stay unwritable until `gram-value` carries them (ADR-014 section 3.6).
- The first release containing this change is 0.3.0, a minor version, since a tag that was dropped is now stored.
