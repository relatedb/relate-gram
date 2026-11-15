# Research: The embedding Tag

## R1. Validation and the profile

- **Decision**: `Tag::Embedding` with name `embedding`; `Tag::validate` is unchanged (it calls `interpret_tag`, strips the `embedding: ` prefix, runs `profile::check`, returns the normal content). `profile::check` gets an arm for `Typed::Embedding`: dimension above 4096 is the first error, then the index of the first non-finite element found by `elements::widen`.
- **Rationale**: ADR-014 sections 2.1 and 2.2; the check order matches gram-value's own (dimension range before content); the scan is one pass, done in `check` so a database is not needed.
- **Alternatives**: scan in the loader only (rejected: `check` must find it, FR-004).

## R2. An array of embeddings

- **Decision**: in the lowering of an `array` node in `src/values.rs`, before the elements are converted, look at the named children. If every child is a `tagged_string` whose tag is `embedding`, push one `value.tag-content` error at the array node and return `None`. Otherwise the elements are converted as today: an embedding mixed with other values converts to a tagged value of tag `embedding`, which differs in kind from its neighbours, so the existing homogeneity check reports `value.array-mixed` at the first differing element (a bad embedding element reports its own `value.tag-content` first, as for other tags).
- **Rationale**: ADR-014 section 2.2 says the array is decided before the elements' own content; the reviewer found that `Tag::ALL` growing would otherwise make a bad element report `value.tag-content` first.
- **Alternatives**: a new code (rejected in the ADR: no new code).

## R3. The vector statement and its dtype

- **Decision**: `TypedGroup` gets `dtype: Option<EmbeddingDtype>`, set for embeddings by re-running `interpret_tag("embedding", content)` on the stored normal content (as a point is re-interpreted today). `typed_param` returns `Value::List` of `Value::Float` from `elements::widen`. `typed_expr` for an embedding returns `vector({var}, size({var}), KIND)` with `KIND` one of `FLOAT16`, `BFLOAT16`, `FLOAT32`, `FLOAT64` as a literal. An embedding is never inside a list (R2), so `group.list` is false for it.
- **Rationale**: ADR-014 section 2.2; a parameter kind would bypass Relate's static preview gate, and the spike showed the dimension may be a parameter or `size()`.
- **Alternatives**: carry the dtype in `Lowered` (rejected: ADR-013 fixes `Lowered { value, span, items }`; one more interpretation per embedding is cheap and measured by SC-006).

## R4. The probe

- **Decision**: `load::write` scans the plan for embeddings whose dtype is `F16` or `BF16` before `write_plan`. If there are none, no probe. Otherwise it runs `RETURN vector([0.0], 1, FLOAT16)`; on `Err(e)` it returns one `value.tag-content` diagnostic per such value (position from `Lowered.span`) with the message "this database does not accept 16-bit vectors: <e>; set RELATE_PREVIEW=float16-coordinate-types", and writes nothing.
- **Rationale**: ADR-014 section 2.2 and the reviewer's finding that the gate is static and quotes no content, so the batch statement could point at one value only.
- **Alternatives**: let the batch statement fail (rejected in ADR-014 3.9).

## R5. Export and the vector's kind

- **Decision**: the exporter's property-value mapping gets a `Value::Vector { coord, values }` arm. The kind is read from `format!("{coord:?}")` (`Float16`, `BFloat16`, `Float32`, `Float64`, `Int8` to `Int64`), because `relate` does not re-export `VectorCoordKind` and Constitution V forbids naming it from `relate-model`. A float kind narrows each `Value::Float` to the kind with `elements::narrow`, builds `Embedding::new(dtype, values.len(), bytes)`, and returns `.to_value()`. An integer kind, or an unknown name, is an error `a vector of kind X has no gram form; the embedding tag carries float vectors only`. A test creates one vector of each kind and checks the mapping, so a renamed variant fails the test.
- **Rationale**: ADR-014 section 2.3; the alternative, `valueType(n.e)` per property, costs a query for each vector.
- **Alternatives**: depend on `relate-model` (rejected: Constitution V).

## R6. 16-bit conversion and its test

- **Decision**: `elements.rs` has `widen(dtype, bytes) -> Vec<f64>` and `narrow(dtype, &[f64]) -> Vec<u8>`; `F32` and `F64` use the standard `from_le_bytes` and `as` conversions; `F16` and `BF16` are hand-written (bf16 is the top 16 bits of an f32; f16 is decoded and encoded by bit fields with round-to-nearest-even, exact for already-representable values). The test `tests/embedding16.rs` runs all 65,536 patterns of each format: for each finite pattern, widen, store in a Relate vector (batched), read back, narrow, compare bytes; non-finite patterns are skipped because Relate rejects them.
- **Rationale**: ADR-014 section 2.4; the stored values are already narrowed, so the narrowing is exact, and the test proves it for Relate's actual storage.
- **Alternatives**: the `half` crate (rejected in the ADR: no new dependency).

## R7. The preview flag in tests

- **Decision**: two integration-test targets; `embedding16.rs` sets `RELATE_PREVIEW=float16-coordinate-types` with `std::env::set_var` in a function every test calls before its first `fresh_db()`, guarded by `std::sync::Once`; `embedding16_off.rs` never sets it. Cargo runs each target as its own process, so the two do not interfere. If Relate reads the variable once per process, one `Once` is enough; if per connection, it works the same.
- **Rationale**: the flag is a process environment variable (spec assumption); the spike ran it that way.

## R8. Fixtures

- **Decision**: valid fixtures take the reference-writer F32, F64 and F16 cases, the non-normal case and the BF16 case from `gram-value`'s `conformance/tags.gram` (copied with a comment naming the case ids and `gram-value` 0.2.0). Invalid fixtures: one per spec scenario of User Story 3 (bad Base64, an `I32` dtype, a lower-case dtype, a 4097-dimension F32 and a NaN F32 generated by a test helper script kept in `tests/support`, an array of embeddings, a mix). A generator for the 4097-dimension and NaN payloads is a small function in `tests/support`, not a binary.
- **Rationale**: the corpus is the conformance reference (gram-value ADR-001); the Relate-specific limits have no corpus case.

## R9. Existing tests that name `embedding`

- **Decision**: `tests/contract/export_values.rs` (lines 136 and 163) uses a property key named `embedding` holding a `FLOAT32` vector to assert that vectors are refused. They change to an integer vector (`INTEGER16`) so they keep testing the refusal, and a new test covers the float case.
- **Rationale**: ADR-014 section 2.3 changes the behavior they pin.

## R10. Performance baseline

- **Decision**: there is no baseline before the feature, because export refuses vectors today. A new `#[ignore]` case in `tests/perf.rs` loads and exports 1,000 vectors of dimension 1,536; its first measurement on the finished feature is recorded in `tasks.md`, and the ceiling is 1.2 times that figure for later changes. T001 re-measures only the typed 10,000-element load, whose ceiling already exists (SC-005).
- **Rationale**: SC-005 is relative to the previous release; SC-006 can only be relative to the feature's own first measurement.
