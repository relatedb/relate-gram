# Quickstart: gram-value as the Value Model

Prerequisite: `git submodule update --init external/tree-sitter-gram`; a built `relate-gram`.

1. **Strings are written canonically.** Load a file with `(:P {a: "active", b: "Ada Lovelace", c: "true", d: "", e: "42"})`, export it:
   ```bash
   printf '(:P {a: "active", b: "Ada Lovelace", c: "true", d: "", e: "42"})\n' > /tmp/s.gram
   relate-gram normalize /tmp/s.gram
   ```
   Expected: `{a: active, b: "Ada Lovelace", c: "true", d: "", e: "42"}`.

2. **Slash strings round-trip.**
   ```bash
   printf '(:P {a: "// x", b: "/** y */ z"})\n' > /tmp/sl.gram
   relate-gram normalize /tmp/sl.gram
   ```
   Expected: the same two strings with no backslash before the leading slash.

3. **`-0` loads.**
   ```bash
   printf '(:P {a: -0, l: [-0]})\n' > /tmp/z.gram
   relate-gram check /tmp/z.gram
   ```
   Expected: no errors; `normalize` writes `0`.

4. **Typed values unchanged.** `relate-gram normalize` of `tests/fixtures/valid/typed-all.gram` writes the tagged strings of ADR-011 and points as `SRID=n;x y [z]`.

5. **Everything else.** `cargo test`, `scripts/check-public-api.sh`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --release --test perf -- --ignored --nocapture`.
