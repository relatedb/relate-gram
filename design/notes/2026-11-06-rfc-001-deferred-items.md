# Items moved out of RFC-001

Two items were listed as open questions in RFC-001 (gram graph support for Relate) during review. Neither is a question about the design, so they are kept here as planning notes. They bind nothing.

## Choosing and updating the gram parser

Findings from pattern-rs (`gram-codec`), read on 2026-11-06:

- The default parser is a hand-written `nom` parser. Its `Value` enum has string, integer, decimal, boolean, array (heterogeneous), integer range and tagged string, with no maps, measurements or hexadecimal and octal literals.
- An optional `cst` feature wraps tree-sitter-gram and returns source spans. It pins a submodule at tree-sitter-gram 0.3.11, while the current grammar is 0.6.1. Its `ArrowKind` records direction only, so `-->` and `==>` are indistinguishable.
- `parse_gram_with_header` treats a leading bare record as the file header.
- `ReconciliationPolicy` there has `Strict`, `FirstWriteWins`, `LastWriteWins` and `Merge`, similar to ADR-001's `on-conflict` options (`error`, `first`, `last`). `Strict` fails on any difference between duplicates, so it cannot be used directly for ADR-001's union-and-conflict rule.

To decide in a parser ADR: which parser path to use, the grammar bump, what the CST must expose (arrow family, `::`, annotations), and whether the loader reuses any of `StandardGraph` or `PatternGraph`. tree-sitter-gram's test corpus is the conformance reference.

## Future direction: ontologies

pattern-rs has a spike that expresses Neo4j's ontology-graph spec in gram (`data/schema/ontology/`). Relate has a read-only ontology graph written through `db.ontology.*` procedures. A gram ontology file could eventually load into it. Out of scope for now.

## Candidate follow-on RFCs

- Identity and named graphs (load-time resolution, merge rules, `@id`, managed multi-graph loading).
- The meta node.
- CLI integration.
- Language bindings (TypeScript, Python) with client-side result structures. Deferred until the Rust core and CLI exist.

Value mapping is RFC-002. ADRs for the meta node format and the named graph format come after their parent RFCs.

## Advanced-phase constructs (from RFC-002)

Which gram constructs the advanced value mapping would make acceptable, and which metadata beyond a format tag is worth recording (for instance units for measurements). Revisit when the advanced mapping is designed.
