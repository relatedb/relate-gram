---
id: ADR-014
title: The embedding Tag Carries Relate Vectors
status: proposed
rfcs: [RFC-002]
created: 2026-11-15
---

# ADR-014: The `embedding` Tag Carries Relate Vectors

## 1. Context

Relate stores a vector as a property: a fixed number of coordinates, all of one kind. The kinds are `INTEGER8` to `INTEGER64`, `FLOAT16`, `BFLOAT16`, `FLOAT32` and `FLOAT64`. Embeddings from language models are stored this way, with hundreds to a few thousand coordinates. A database that holds one cannot be exported to gram today, because the exporter rejects every vector (ADR-011 2.8), and a gram file cannot load one. The loader drops the tag if a file uses `embedding`, and says so.

ADR-011 3.6 rejected a tag for vectors because a vector is a long list of numbers that a person cannot read or write, and readable, writable text is what gram is for. `gram-value` 0.2 (the shared model of gram's values, adopted in ADR-012 and ADR-013) now defines a known tag `embedding` for machine-made vectors and does not claim they are readable. Its content is Base64 of a restricted SafeTensors buffer (Hugging Face's tensor file format) holding one rank-one tensor, with an explicit dtype and dimension. The accepted dtypes are `F16`, `BF16`, `F32` and `F64`; integer and bit-packed vectors are deferred in `gram-value`'s ADR-008. Content with an unknown or malformed form is an error with the codes `invalid-form`, `out-of-range` or `unsupported`.

The four float dtypes match four of Relate's coordinate kinds. A spike against `relate` 1.0.1 (`design/notes/2026-11-15-vector-properties-in-relate.md`) found what Relate adds:

- A vector is built in Cypher with `vector($list, dim, KIND)`, and reads back as a `Value::Vector` whose elements are floats already rounded to the kind (a `FLOAT32` element widens to f64 and narrows back exactly).
- `FLOAT32` and `FLOAT64` work by default. `FLOAT16` and `BFLOAT16` fail unless the environment variable `RELATE_PREVIEW=float16-coordinate-types` is set.
- Relate rejects NaN and infinities, which `gram-value` accepts in an embedding's bytes.
- Relate's dimension is 1 to 4096; `gram-value`'s is 1 to 2^20.

## 2. Decision

### 2.1 `embedding` is a ninth known tag

`embedding` joins the eight tags of ADR-011 2.2 in `typed::Tag`. The tag name is lower case and matched exactly, like the others. Content is validated with `gram_value::tags::interpret_tag`, with its diagnostics shown as `value.tag-content` errors (ADR-012 2.2), so `check` validates embeddings without a database.

### 2.2 Load

A recognized `embedding` becomes a Relate vector of the matching kind: `F32` is `FLOAT32`, `F64` is `FLOAT64`, `F16` is `FLOAT16` and `BF16` is `BFLOAT16`. The loader decodes the payload's elements to f64, which is exact for every dtype, and builds the vector with `vector(list, dim, KIND)` in a follow-up statement for each key and dtype in a batch (ADR-011 2.7). `embedding` is not recognized inside an array: a list of vectors is not stored by Relate, so an array of `embedding` values is a `value.array-mixed` error naming the tag.

Two rules are Relate's, and the database-free `check` enforces both:

- A non-finite element is a `value.tag-content` error naming the first element's index, because Relate cannot store it.
- A dimension above 4096 is a `value.tag-content` error stating Relate's limit and that the tag itself allows more.

A 16-bit dtype loads only when Relate's preview flag is on, which depends on the environment and not on the file. The loader does not set the flag. A refusal by Relate to build the vector fails the load, rolls it back, and is reported at the first value of that group, with Relate's message, which names the flag (the existing report for a failed typed statement).

### 2.3 Export

A vector of a float kind is written as an `embedding` tagged value. The exporter narrows each stored element to the vector's own kind, builds the payload as little-endian bytes of that dtype, and writes `gram_value::tags::Embedding::to_value`, so the text is gram-value's normal form and any producer's content round trips to the same bytes. Export needs no preview flag. It writes the vector as a property value, so a vector inside a list is never produced.

A vector of an integer kind remains an `export.value` error. The message names the element and key and says that the `embedding` tag carries float vectors only, so the user knows why the integer kind is refused and which kinds are not.

### 2.4 No fix

A bad `embedding` has no correction: the intended numbers are not known, as for the other tags' content errors (ADR-010 4). Dropping a tag is not offered, because an `embedding` is recognized and an unrecognized one would be a different value.

### 2.5 Amendments

ADR-011 2.8 and 3.6 stand for integer vectors only. The profile list in ADR-011 2.9 loses "no vectors", `docs/supported-gram.md` lists nine tags, and the scope line of `CLAUDE.md` changes from "Vectors cannot be written in gram" to the float vectors that `embedding` carries.

## 3. Alternatives Considered

3.1. **Keep vectors unwritable.** Rejected: the original reason, that a person cannot read the numbers, does not apply to a tag that states it is machine output, and the cost is that a database with embeddings cannot be exported or round-tripped.

3.2. **A gram array of decimals as the carrier.** Rejected: every element becomes a 64-bit decimal, the dtype is lost, and a 4096-element vector becomes tens of kilobytes of text a person still cannot read. The array also cannot be told apart from an ordinary list property on load.

3.3. **A relate-gram tag such as `vector`.** Rejected: it duplicates what `embedding` already states in a form other gram tools will not recognize, and ADR-012's reason for a shared tag vocabulary applies.

3.4. **Store NaN and infinities by other means (a wider kind, or drop the element).** Rejected: no kind holds them, and dropping an element changes the vector's meaning without the file saying so.

3.5. **Enable the 16-bit preview flag from the loader.** Rejected: the flag is a decision about the caller's environment, and a library that sets it would change the behavior of that process's other queries.

3.6. **Carry integer vectors with a second tag.** Rejected: `gram-value` has deferred quantized and packed codes until it can state their scale and packing; transporting codes without them would be misleading.

## 4. Consequences

- A database with float vectors exports, and a file with `embedding` values loads, so a round trip through gram keeps embeddings. A file that used `embedding` as a free tag now loads values that were dropped before: a behavior change, released as a minor version.
- Text size grows: a 1,536-dimension `FLOAT32` vector is about 8.3 KB, and one of 4096 about 22 KB, against 8 bytes a coordinate in memory. The loader and exporter each hold the payload bytes once more than a plain scalar; `tests/perf.rs` gets a vector case and the perf ceilings (SC-006) are re-checked.
- 16-bit vectors load only on a database started with the preview flag, and the failure message names it. When Relate stabilizes the kind, the rule drops out without a change to this decision.
- Integer vectors remain unexportable, and a database with them still fails the export with each one listed. This waits on `gram-value`.
- A file that is valid for `gram-value` can fail to load here (a NaN, or a dimension above 4096), with the limit named. These are Relate's limits, not the tag's.
- Follow-up work: a spec for the implementation (fixtures from `gram-value`'s `conformance/tags.gram`, load, export and round-trip tests for each dtype, a perf case); the documentation and changelog entries of 2.5; the version to release.

## 5. Related

- RFC-002 (the value mapping).
- ADR-011 (typed tags; 2.8 and 3.6 amended here), ADR-012 and ADR-013 (`gram-value`), ADR-006 (the mapping), ADR-010 (fixes).
- `gram-value` 0.2, its ADR-008 (the `embedding` tag); `design/notes/2026-11-15-vector-properties-in-relate.md`.
