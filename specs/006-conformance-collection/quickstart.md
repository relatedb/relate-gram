# Quickstart: The Conformance Collection

Prerequisites: `git submodule update --init external/tree-sitter-gram`; the `gram` tool (`cargo install gram-data --locked`) for the lint.

1. **Run the collection.**
   ```bash
   cargo test --test conformance                      # preview flag unset
   cargo test --test conformance16                    # preview flag set
   ```
   Expected: every case `passed`, `skipped` or `not-applicable`; results in `target/conformance/flag-unset.gram` and `flag-set.gram`.

2. **See a case fail.** Change one `line` in `conformance/reject/rel-no-type/case.gram` and run step 1: the case fails and names the part that differs. Restore it.

3. **Check the collection.**
   ```bash
   CONFORMANCE_REQUIRE_LINT=1 cargo test --test conformance_checks
   gram lint --strict conformance/reject/rel-no-type/case.gram
   ```

4. **Read the form.** `conformance/README.md` states the case form, the value table, the vocabulary and the capability probes.

5. **Everything else.** `cargo test`, `scripts/check-public-api.sh`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`.
