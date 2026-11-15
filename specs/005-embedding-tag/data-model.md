# Data Model: The embedding Tag

No stored entity beyond Relate's own vector property.

- **Embedding value** (`gram_value::tags::Embedding`): dtype (`F16`, `BF16`, `F32`, `F64`), dimension (1 to 4096 here, 1 to 2^20 in the tag), payload bytes (little-endian, `dimension * width`). In the loader it is a `Value::Tagged("embedding", normal_content)` with its position.
- **Vector property** (`relate::Value::Vector { coord, values }`): `coord` is one of eight kinds, `values` are floats (or integers for integer kinds) already rounded to the kind; the dimension is `values.len()`.
- **Probe**: the statement `RETURN vector([0.0], 1, FLOAT16)`, run once before any write when the load holds a 16-bit embedding.
- **Mapping** (ADR-014 sections 2.2 and 2.3):

| dtype | Relate kind | Default | Elements |
|---|---|---|---|
| `F32` | `FLOAT32` | on | f32 widened to f64 |
| `F64` | `FLOAT64` | on | f64 |
| `F16` | `FLOAT16` | needs `RELATE_PREVIEW=float16-coordinate-types` | f16 widened exactly |
| `BF16` | `BFLOAT16` | needs the same flag | bf16 widened exactly |
| none | `INTEGER8` to `INTEGER64` | n/a | export error |

- **Profile errors** (all `value.tag-content`): gram-value's three codes, in order; a dimension above 4096; the first non-finite element's index; an array of embeddings; a 16-bit dtype when the probe fails.
- **Export mapping**: float vector, kind from the `Debug` name of `coord`, elements narrowed to the kind, `Embedding::new(...).to_value()`; integer vector or unknown name is an `export.value` error.
