---
id: SPIKE-007
title: A Typed Syntax Layer over tree-sitter-gram with gram-value Values
status: done
rfcs: [RFC-005]
created: 2026-11-15
---

# SPIKE-007: A Typed Syntax Layer over tree-sitter-gram with gram-value Values

## 1. Question

SPIKE-006 found that the pattern-rs readers lose or change what a gram file says, and that gram's own documentation expects a consumer that needs arrow kinds or positions to keep the syntax tree from the parse phase. Is a layer that lowers the tree-sitter-gram syntax tree into typed elements with `gram_value::Value` leaves, with positions and diagnostics, small enough to be worth building, and does it carry what a strict consumer needs? The consumer tested is relate-gram's own front half.

## 2. Method

A Rust crate, `prototype/` (`gramsyn`), built on `tree-sitter` 0.25, `tree-sitter-gram` 0.7.0 and `gram-value` 0.2:

- `syntax.rs` and `lower.rs`: a typed document (header record, patterns in their written forms, annotations, doc comments) with byte spans everywhere, arrow family and direction, `:` versus `::`, repeated keys kept, values as `gram_value::Value`s, and diagnostics for what is wrong with gram itself (syntax errors, repeated keys, bad content of a known tag).
- `project.rs`: the `Pattern<Subject>` view as gram documents it (a path is `[ | [r1 | a, b], [r2 | b, c] ]`, a left arrow reverses its elements, an annotation is a one-element pattern), with each pattern's origin span, and the diagnostic that needs it (an identified element defined twice).
- `tests/inputs.rs`: the gaps of SPIKE-006, run against that spike's inputs.
- `tests/relate_policy.rs`: relate-gram's strict-subset rules written as a policy over the typed document only, run over all 71 invalid fixtures of relate-gram and compared with their `.expect` files by code, line, column and byte offset. It reuses relate-gram's `typed::Tag::validate` for the Relate profile of known tags.

## 3. Time-box

Four hours.

## 4. Findings

- **Every gap of SPIKE-006 is closed in the typed layer.** Repeated keys are kept and reported with the first as related; a fenced tag is kept; arrow family and direction are kept (`==>`, `~~>`, `--`, `<-->` are distinct); `::` before a label or key is read and marked; an integer beyond 64 bits and a decimal beyond a float are held as gram-value's `big_integer` and `big_decimal`; an annotation's map value is a map; `{}` is an empty header record; a doc comment is kept.
- **The consumer test passes.** Of the 71 invalid fixtures, 67 are reproduced exactly (same code, line, column and byte offset) by a policy of about 190 lines that reads only the typed document; 4 are out of scope: three identity rules that need the graph-level resolver (`id.kind-conflict`, `id.endpoints-differ`, `id.rel-never-defined`) and one invalid-UTF-8 input (the prototype takes text, not bytes).
- **The first run reproduced 60 of the 67, and the 7 misses taught what the layer needs beyond the first design:**
  - *Positions of tokens and sub-parts.* `construct.double-colon` points at the `::` token and `id.duplicate-definition` points at the identifier, not at the enclosing pattern. The typed layer records the span of each colon token, and the pattern projection records the span of the identifier. A source map that holds only a pattern's span is not enough.
  - *The grammar node a value came from.* `value.octal` is a rejection of a spelling, and `Value` does not remember it (`010` is the integer 8). Each lowered value keeps its grammar node kind (`octal`, `hexadecimal`, `tagged_string`, `range`, ...).
  - *Precedence between policy rules.* Relate reports a missing relationship type only for an arrow with no other error. That is the consumer's rule and belongs in the policy.
  - *A Relate rule that is not about gram:* an array of embeddings (ADR-014).
- **Gaps found in gram-value:**
  - There is no way to build a `big_integer` from hexadecimal or octal text, because `integer_from_digits` takes decimal digits. A hexadecimal literal beyond 64 bits is reported as out of range here.
  - `Value::map` reports a repeated key without a position, as designed, so the reader detects repeats itself.
- **Size.** The layer is 950 lines (the model 197, the lowering 556, the projection 176, the entry point 21). The same work today is spread over relate-gram's `syntax.rs`, `values.rs` and `plan.rs` (1,236 lines), which also hold Relate policy and the identity planner, so the two numbers are not a like-for-like comparison.
- **Speed.** Parsing and lowering a 3.1 MB file of 50,000 nodes with properties takes 231 ms in a release build; the projection takes 10 ms.
- **pattern-rs's path structure differs from the grammar documentation.** The documentation says a path is the flat pattern of its relationships; pattern-rs reads `(a)-[:R]->(b)-[:S]->(c)` as a relationship `S` whose first element is the relationship `R` and whose second is `c`. The prototype follows the documentation. I have not checked which one pattern-hs, the reference, produces.

## 5. Conclusion

A typed layer over the tree-sitter-gram tree, with gram-value values, byte spans on every part and node kinds retained, is small and carries what relate-gram's strict subset needs: 67 of 71 rejections reproduce from the typed document alone, and the other 4 need pieces (a graph-level resolver, byte input) that sit outside the layer. The design that the exercise supports is the one discussed: a decorated syntax tree with positions of tokens and identifiers, a `Pattern<Subject>` projection with a source map, diagnostics for gram itself in the layer, and consumer policy outside it.

What the prototype does not show: a second language, a shared corpus for the layer, the graph-level resolver for identity, or the full gram grammar (a hand-written walk over the node kinds this spike met; the grammar has more). Where the layer should live, and whether it is one Rust core or a native implementation per language, is not settled by this spike.

## 6. Reproducing

```bash
cd prototype
cargo test                                   # the SPIKE-006 gaps and the relate-gram policy test
cargo test --release --test timing -- --ignored --nocapture
```

The policy test prints a per-code table of fixtures reproduced.
