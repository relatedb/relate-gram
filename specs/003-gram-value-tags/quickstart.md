# Quickstart: Known Tags from gram-value

Prerequisite: `git submodule update --init external/tree-sitter-gram`.

1. **Points in the general form load and round-trip.**
   ```bash
   printf '(:Place {at: point`SRID=4326;12.5 55.7`, alt: point`SRID=4979;1 2 3`})\n' > /tmp/place.gram
   relate-gram check /tmp/place.gram
   ```
   Expected: no errors. Loading into a database and exporting writes the same two points back.

2. **Old spelling and a missing SRID are rejected.**
   ```bash
   printf '(:P {a: point`SRID=4326;POINT(1 2)`})\n' > /tmp/old.gram
   printf '(:P {a: point`30 10`})\n' > /tmp/nosrid.gram
   relate-gram check /tmp/old.gram /tmp/nosrid.gram
   ```
   Expected: `value.tag-content` at each tagged string; the second message says Relate needs an SRID.

3. **A time without seconds loads; without an offset it does not.**
   ```bash
   printf '(:E {t: time`17:35Z`})\n' > /tmp/t1.gram
   printf '(:E {t: time`17:35`})\n' > /tmp/t2.gram
   relate-gram check /tmp/t1.gram /tmp/t2.gram
   ```
   Expected: the first passes, the second fails with `value.tag-content` naming the offset.

4. **`week` is an unrecognized tag.**
   ```bash
   printf '(:E {w: week`2024-W05`})\n' > /tmp/w.gram
   relate-gram check /tmp/w.gram
   ```
   Expected: a warning that the tag was dropped.

5. **Everything else.** `cargo test`, `scripts/check-public-api.sh`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --release --test perf -- --ignored --nocapture` against the recorded baseline.
