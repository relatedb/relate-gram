# CLAUDE.md

## Purpose

`relate-gram` is a standalone, out-of-tree Rust crate that adds a strict subset of the
[gram](https://github.com/gram-data/gram-spec) notation to [Relate](https://relatedb.ai/) (the embedded Cypher graph
database in `neo4j-labs/cypherlite`): load a gram file into an in-memory Relate database, and export one back out.
It links the public `relate` crate as a library and integrates minimally with the `relate` CLI (`relate run`,
ADR-0429/0431 upstream). It may be brought in-house later.

## Scope (decided in the kickoff conversation)

- Gram graph notation only: nodes, relationships, paths, and the top-level record. No annotations, no pattern notation.
- **Strictness rule:** reject anything that does not map directly to Cypher, with a located error. Property values are
  string, integer, float, boolean and homogeneous arrays of scalars; no nested maps, tagged strings, ranges or
  measurements. Nodes may have several labels; a relationship has exactly one type.
- The top-level record maps to a **meta node**; its format is to be set by an ADR.
- **Identity:** gram IDs only resolve references while loading, and are dropped. The first appearance of an ID fixes it
  as a node or a relationship; using it later as the other kind is an error. A repeated appearance merges by default.
- Advanced: one Relate database holds several graphs, distinguished by an `@id` namespace (format to be set by an ADR).
  The `@id` property round-trips to gram by becoming the element identifier. Also: managed multi-graph load, and merging
  graphs within one namespace.
- Parser: `gram-codec` (pattern-rs), updated as needed to match current tree-sitter-gram (e.g. scalar arrays inside a
  shallow map); tree-sitter-gram's corpus is the conformance reference.

- **Export IDs:** modes `auto` (default), `generate`, `property`. `auto` uses `@id` if present, else a configured
  id-property if present and unique, else a sequential ID (`n0…` for nodes, `r0…` for relationships, deterministic
  order). If the id-property is missing on some elements or not unique, export errors and lists the offenders (no silent
  fallback). The property stays in the exported record. Non-symbol identifiers are quoted automatically.

Still open: design constraints for the TypeScript and Python bindings.

## Reference repos (local, read-mostly)

- `/Users/abk/Developer/neo4j-labs/cypherlite`: Relate. Its `AGENTS.md` and `extensions/` show the extension contract.
- `/Users/abk/Developer/relateby/pattern-rs`: `gram-codec` and related crates.
- `/Users/abk/Developer/gram-data/tree-sitter-gram`: the grammar and its test corpus.

## Design process: RFC → ADR → implementation (with spikes)

Adapted from `/Users/abk/Developer/akollegger/graph-web/design/`. Skills live in `.agents/skills/` and are symlinked
from `.claude/skills/`.

- **RFC** (`design/rfc/RFC-NNN-*.md`, `/rfc-create`, `/rfc-review`): the WHAT and WHY, with candidate directions. No implementation detail.
- **Spike** (`design/spikes/SPIKE-NNN-*/SPIKE.md`, `/spike-create`): a time-boxed investigation of one empirical question.
- **ADR** (`design/adr/ADR-NNN-*.md`, `/adr-create`, `/adr-review`): one technical decision, always with a parent RFC.
- **Notes** (`design/notes/YYYY-MM-DD-*.md`): freeform, non-binding.
- Don't hand-edit the index tables in the `design/*/README.md` files; the skills maintain them.
- Implementation runs through Spec Kit (`/speckit-specify`, `-plan`, `-tasks`, `-implement`), installed in `.specify/`. Specs
  implement accepted ADRs and name them rather than restating them; the ADR wins on conflict. Commits and PRs cite the ADR.
  The constitution (`.specify/memory/constitution.md`) is still the unfilled template.
- Write design documents for a newcomer: name project artifacts on first use.
