# Quickstart: The embedding Tag

Prerequisites: `git submodule update --init external/tree-sitter-gram`; a built `relate-gram`; `gram-value` 0.2.

1. **An embedding loads as a vector.** The reference-writer F32 case of gram-value (two elements, `1.0` and `-2.0`):
   ```bash
   printf '(:P {id: 1, e: embedding`QAAAAAAAAAB7ImVtYmVkZGluZyI6eyJkdHlwZSI6IkYzMiIsInNoYXBlIjpbMl0sImRhdGFfb2Zmc2V0cyI6WzAsOF19fSAgAACAPwAAAMA=`})\n' > /tmp/e.gram
   relate-gram check /tmp/e.gram
   relate-gram normalize /tmp/e.gram
   ```
   Expected: no errors; `normalize` writes the same `embedding` content.

2. **Limits are found without a database.** An embedding of dimension 4097, one with a NaN, and an array of embeddings each fail `check` with `value.tag-content` and a position (fixtures `tests/fixtures/invalid/embedding-*.gram`).

3. **Integer vectors are refused on export.** Create `CREATE (:P {e: vector([1, 2], 2, INTEGER16)})` in a database and export it: `export.value` naming the property and saying the tag carries float vectors only.

4. **16-bit kinds.** With `RELATE_PREVIEW=float16-coordinate-types` set, an `F16` or `BF16` file loads and exports unchanged. Without it, `load` stops before writing and reports each value; `check` still passes.

5. **Everything else.** `cargo test` (both 16-bit targets run), `scripts/check-public-api.sh`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --release --test perf -- --ignored --nocapture`.
