# Quickstart: validating typed tags

Prerequisites: `cargo build`, and `git submodule update --init external/tree-sitter-gram`.

1. **Load and query typed values**

   ```bash
   cat > /tmp/typed.gram <<'GRAM'
   (ada:Person {born: date`1815-12-10`, id: uuid`123e4567-e89b-12d3-a456-426614174000`,
                loc: point`SRID=4326;POINT(-0.1278 51.5074)`, span: duration`P1Y2M`})
   GRAM
   target/debug/relate-gram load /tmp/typed.gram --db /tmp/typed-db
   relate query --database /tmp/typed-db "MATCH (n:Person) RETURN valueType(n.born), valueType(n.loc), n.span"
   ```

   Expected: DATE, POINT, and the duration `P1Y2M` (spec User Story 1).

2. **Round trip**

   ```bash
   target/debug/relate-gram export --db /tmp/typed-db > /tmp/typed-out.gram
   target/debug/relate-gram load /tmp/typed-out.gram --db /tmp/typed-db2
   ```

   Expected: the export shows the same four values as tagged strings; the second database holds equal values (User Story 3, SC-001).

3. **Bad content has a position**

   ```bash
   printf '(a {when: date`2024-13-45`})\n' > /tmp/bad.gram
   target/debug/relate-gram check /tmp/bad.gram
   ```

   Expected: `error[value.tag-content]` at line 1, with a marker under the tagged string, and exit status 1 (User Story 2).

4. **Automated**

   ```bash
   cargo test
   cargo test --release --test perf -- --ignored --nocapture
   scripts/check-public-api.sh
   ```

   Expected: all pass; the typed-density perf case stays within three times the plain case (SC-004).
