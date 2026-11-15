---
id: ADR-014
title: The embedding Tag Carries Relate Vectors
status: proposed
rfcs: [RFC-002]
created: 2026-11-15
---

# ADR-014: The `embedding` Tag Carries Relate Vectors

## 1. Context

Relate stores a vector as a property: a fixed number of coordinates, all of one kind. The kinds are `INTEGER8` to `INTEGER64`, `FLOAT16`, `BFLOAT16`, `FLOAT32` and `FLOAT64`. Embeddings from language models are stored this way, with hundreds to a few thousand coordinates. A database that holds one cannot be exported to gram today, because the exporter rejects every vector, and a gram file cannot load one. If a file uses the tag `embedding`, the loader drops the tag and says so.

An earlier decision (ADR-011 3.6) rejected a tag for vectors because a vector is a long list of numbers that a person cannot read or write, and readable, writable text is what gram is for. `gram-value` 0.2, the shared model of gram's values, now defines a known tag `embedding` for machine-made vectors and does not claim they are readable. Its content is Base64 of a restricted SafeTensors buffer (Hugging Face's tensor file format) holding one rank-one tensor, with an explicit dtype and dimension. The accepted dtypes are `F16`, `BF16`, `F32` and `F64`; integer and bit-packed vectors are deferred by `gram-value`. Malformed content is reported with the codes `invalid-form`, `out-of-range` or `unsupported`.

The four float dtypes match four of Relate's coordinate kinds. A spike against `relate` 1.0.1 (`design/notes/2026-11-15-vector-properties-in-relate.md`) and a check of its source found what Relate adds:

- A vector is built in Cypher with `vector($list, dim, KIND)`, and reads back as a `Value::Vector` whose elements are floats already rounded to the kind. Widening a stored element to f64 and narrowing it back is exact.
- `FLOAT32` and `FLOAT64` work by default. `FLOAT16` and `BFLOAT16` fail unless the environment variable `RELATE_PREVIEW=float16-coordinate-types` is set. The refusal is decided from the statement text when the kind is written as a literal, so it happens with zero rows, and its message quotes no content.
- Relate rejects NaN and infinities, which `gram-value` accepts in an embedding's bytes.
- Relate's dimension is 1 to 4096; `gram-value`'s is 1 to 2^20.
- Relate does not store a list of vectors (`InvalidPropertyType`).

## 2. Decision

### 2.1 `embedding` is a ninth known tag

`embedding` joins the eight tags of ADR-011 2.2 in `typed::Tag`. The tag name is lower case and matched exactly, like the others. Content is validated with `gram_value::tags::interpret_tag`, and its three codes are shown as `value.tag-content` errors (ADR-012 2.2), so `check` validates embeddings without a database. No new diagnostic code is added.

### 2.2 Load

A recognized `embedding` becomes a Relate vector of the matching kind: `F32` is `FLOAT32`, `F64` is `FLOAT64`, `F16` is `FLOAT16` and `BF16` is `BFLOAT16`. The loader decodes the payload's elements to f64 (exact for every dtype) and passes them as a list parameter. It builds the vector in a follow-up statement per element kind, key and dtype in a batch (ADR-011 2.7), with the kind written as a literal keyword in the statement text and the dimension as `size(list)`. A kind passed in any other way would bypass Relate's preview gate.

Three rules are Relate's, and the database-free `check` enforces the first two, in this order after `gram-value`'s own checks:

- A dimension above 4096 is a `value.tag-content` error that states Relate's limit and that the tag itself allows more.
- A non-finite element is a `value.tag-content` error that names the index of the first one.
- A 16-bit dtype loads only when Relate's preview flag is on, which depends on the environment and not on the file, so `check` cannot know it. The loader does not set the flag. Before it writes anything, if the load holds any `F16` or `BF16` embedding, it runs one probe (`RETURN vector([0.0], 1, FLOAT16)`). If the probe fails, the load stops with nothing written, and reports a `value.tag-content` error at every such value that names the flag, its environment variable and Relate's message.

`embedding` is not recognized inside an array, because Relate cannot store a list of vectors. An array whose elements are `embedding` values is a `value.tag-content` error at the array: "an array of embeddings has no Relate type; write one property per vector". This is decided while lowering the array, before the elements' own content is checked, so one error is reported. `value.array-mixed` keeps its meaning of an array that holds two kinds or two tags, and an array that mixes an `embedding` with another value still gets it.

### 2.3 Export

A vector of a float kind is written as an `embedding` tagged value. The exporter narrows each stored element to the vector's own kind, builds the payload as little-endian bytes of that dtype, and writes `gram_value::tags::Embedding::to_value`, so the text is `gram-value`'s normal form and content from any producer round-trips to the same bytes. Export needs no preview flag.

A vector of an integer kind remains an `export.value` error. The message names the element and key and says that the `embedding` tag carries float vectors only.

### 2.4 Converting 16-bit elements

The loader widens `F16` and `BF16` elements and the exporter narrows them with conversion code in `src/typed/`, written for the purpose, with no new dependency. A test covers every finite 16-bit bit pattern of both formats, subnormals and negative zero included: the bytes widen to f64, go through a Relate vector and come back as the same bytes. It runs with the preview flag set.

### 2.5 No fix

A bad `embedding` has no correction, because the intended numbers are not known, as for the other tags' content errors (ADR-010 4). No fix removes the tag: a recognized `embedding` with bad content has no plain-string reading, unlike an unrecognized tag. ADR-010's table gains a row saying so.

### 2.6 Amendments

- ADR-011 2.8 and 3.6 stand for integer vectors only, and the passages that call vectors unwritable (the opening paragraph, 2.1's list of storable values, 2.9's profile list and the consequence about vectors) say so.
- `docs/supported-gram.md` lists nine tags, drops `vector` from its example of an unrecognized tag, replaces the sentence that vectors cannot be written, and adds the rejections of 2.2 (a dimension above 4096, a non-finite element, an array of embeddings, a 16-bit dtype without the flag) with a note that `check` cannot know the flag.
- `CLAUDE.md`'s scope line changes from "Vectors cannot be written in gram" to the float vectors `embedding` carries.
- RFC-002 question 7.3 is resolved for vectors, with byte arrays still open.

## 3. Alternatives Considered

3.1. **Keep vectors unwritable.** Rejected: the original reason, that a person cannot read the numbers, does not apply to a tag that states it is machine output, and the cost is that a database with embeddings cannot be exported or round-tripped.

3.2. **A gram array of decimals as the carrier.** Rejected: every element becomes a 64-bit decimal, the dtype is lost, and a 4096-element vector becomes tens of kilobytes of text that cannot be told apart from an ordinary list property on load.

3.3. **A relate-gram tag such as `vector`.** Rejected: it duplicates what `embedding` already states in a form other gram tools do not recognize, and ADR-012's reason for a shared tag vocabulary applies.

3.4. **Store NaN and infinities by other means (a wider kind, or drop the element).** Rejected: no kind holds them, and dropping an element changes the vector's meaning without the file saying so.

3.5. **Enable the 16-bit preview flag from the loader.** Rejected: the flag is a decision about the caller's environment, and a library that sets it would change the behavior of that process's other queries.

3.6. **Carry integer vectors with a second tag.** Rejected: `gram-value` has deferred quantized and packed codes until it can state their scale and packing, and transporting codes without them would be misleading.

3.7. **Load a 16-bit payload as `FLOAT32` when the flag is off.** Rejected: it stores a different kind than the file states, and a later export writes `F32`, so the file changes silently.

3.8. **Export integer vectors as `F64` embeddings.** Rejected: the values are exact, but loading the file gives a float vector, so the kind does not round trip.

3.9. **Let Relate's refusal of a 16-bit kind surface from the batch statement.** Rejected: the message quotes no content, so the load would point at one value of many after the nodes and relationships were written and rolled back. The probe before any write reports every affected value and does less work.

## 4. Consequences

- A database with float vectors exports, and a file with `embedding` values loads, so a round trip through gram keeps float embeddings. For `F16` and `BF16` the load needs the preview flag on. A file that used `embedding` as a free tag now loads values that were dropped before, which is a behavior change released as 0.3.0.
- Text takes about 1.33 bytes per payload byte plus about 80 bytes of header: a 1,536-dimension `F32` vector is about 8.3 KB and a 4096-dimension one about 22 KB. The loader decodes each payload once into a list and the exporter builds each once from the stored elements. The perf follow-up is a vector case in `tests/perf.rs`, with the ceilings (SC-006) re-checked.
- `check` and `load` can disagree about a 16-bit embedding: `check` does not know the preview flag. When Relate stabilizes the kinds, the rule drops out without a change to this decision.
- Integer vectors remain unexportable, and a database with them still fails the export with each one listed. This waits on `gram-value`.
- A file that is valid for `gram-value` can fail to load here (a NaN, a dimension above 4096, an array of embeddings), with the limit named. These are Relate's limits, not the tag's.
- `check` decodes every element of an embedding to find a non-finite one, a single pass over up to 2^20 elements.
- Follow-up work: a spec for the implementation (fixtures from `gram-value`'s `conformance/tags.gram`, load, export and round-trip tests for each dtype, the exhaustive 16-bit test, a perf case); the documentation and changelog entries of 2.6.

## 5. Related

- RFC-002 (the value mapping; question 7.3).
- ADR-011 (typed tags; 2.8 and 3.6 amended here), ADR-012 and ADR-013 (`gram-value`), ADR-006 (the mapping), ADR-010 (fixes).
- `gram-value` 0.2, its ADR-008 (the `embedding` tag); `design/notes/2026-11-15-vector-properties-in-relate.md`.
