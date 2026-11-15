# What Relate does with vector properties (spike for the embedding tag)

Date: 2026-11-15. Checked against `relate` 1.0.1 through its public API (an in-memory database and
`Connection::query_with_params`), to inform an ADR that carries Relate vectors in gram's `embedding` tag
(gram-value 0.2, its ADR-008). Freeform and non-binding.

## Findings

- **Writing.** A vector is made in Cypher: `vector($list, dim, KIND)`. The loader's batched queries already
  pass parameters, so `SET n.e = vector(row.xs, 3, FLOAT32)` fits. A `Value::Vector` can also be passed as a
  parameter or in a property map (`SET n = $props`), so the test helper `create_node` works for exports.
- **Reading.** A vector property comes back as `Value::Vector { coord, values }` with every element a
  `Value::Float` (or `Value::Int` for integer kinds). The values are the stored, already narrowed numbers:
  `0.1` in `FLOAT32` reads back as `0.10000000149011612`, so widening f32 to f64 and narrowing back is exact.
- **Kinds.** `FLOAT32` and `FLOAT64` work by default. `FLOAT16` and `BFLOAT16` fail with
  `preview feature 'float16-coordinate-types' is not enabled` unless `RELATE_PREVIEW=float16-coordinate-types`
  is set; with it they work and read back narrowed (`0.1` becomes `0.0999755859375` in float16 and
  `0.10009765625` in bfloat16). A value outside float16's range (`70000`) is `InvalidVectorCoordinates`.
- **Non-finite elements.** NaN and infinities are rejected: `vector() coordinates must be finite numbers`.
  gram-value accepts them in an embedding's bytes.
- **Dimension.** Relate allows 1 to 4096 (`vector() dimension is outside the range 1 to 4096`); gram-value
  allows up to 2^20. A literal dimension above the range is a syntax error, a parameter one a semantic error.
- **Integer kinds** (`INTEGER8` to `INTEGER64`) exist and read back as `Value::Int`; the embedding tag does
  not carry them.

## What it means for the ADR

- Load and export of `F32` and `F64` embeddings need nothing special. `F16` and `BF16` depend on a preview
  flag: a load must say so, with a located error, when the flag is off, and an export of such a vector works
  whenever the database holds one.
- Two limits need loader errors beyond gram-value's checks: a non-finite element and a dimension above 4096.
- Export must rebuild the bytes from the stored f64 values (narrowing to the vector's own kind), which is
  exact because the values were narrowed on the way in.
- Open for the ADR: whether the loader sets the preview flag itself (no: it belongs to the caller's
  environment) and how the error names it.
