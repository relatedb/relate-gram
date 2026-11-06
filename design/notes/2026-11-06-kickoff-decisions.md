# Kickoff decisions

Notes from the first conversation about `relate-gram`, a standalone Rust crate that loads and exports a strict subset
of gram (nodes, relationships, paths and the top-level record) for the Relate graph database. These decisions are
direction, not binding; RFCs and ADRs will settle them.

- Standalone out-of-tree crate linking `relate` as a library; minimal `relate` CLI integration at first; possibly
  brought in-house later.
- Parser: `gram-codec`, updated to match current tree-sitter-gram.
- Strictness: reject whatever does not map directly to Cypher. Top-level record becomes a "meta node" (ADR needed).
- Identity: gram IDs resolve references only and are dropped; first appearance fixes node vs. relationship; repeats
  merge by default; reuse as the other kind is an error.
- Namespaces: several graphs in one database, distinguished by an `@id` namespace (ADR needed).
- Export IDs: modes `auto` (default), `generate`, `property`. `auto` = `@id`, else configured id-property if present and
  unique, else sequential `n0`/`r0`. Missing or non-unique id-property is an error listing offenders. The property stays
  in the record. Non-symbol identifiers are quoted.
- Open: TypeScript and Python binding constraints.
