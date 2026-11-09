# ADRs — Architecture Decision Records

ADRs capture the technical **HOW** for one architecturally-significant decision — the Develop
half of the double-diamond design process described in the project's
[CLAUDE.md](../../CLAUDE.md). Every ADR requires at least one existing parent
[RFC](../rfc/README.md) — usually one, but an ADR MAY serve more than one RFC when a decision is
genuinely shared infrastructure (e.g. an identity model that the loader, the exporter and the language bindings
depend on); an ADR never writes implementation code itself.

## Audience

Same reader as the [RFCs](../rfc/README.md#audience): technically fluent, comfortable with
graph databases and the gram notation as general ideas, and new to this project's specifics. An ADR necessarily
carries more of those specifics than an RFC — so name each one on first use ("meta node" is the
node that holds a gram file's top-level record; "named graph" is one of several graphs inside one database) rather than writing as
though the reader has already been through the pipeline.

Context should open on the technical forces, not on the paperwork: what situation makes a decision
necessary, before any parent-RFC citation. `/adr-review` criterion 8 checks this, and
`adr-create`'s "Voice and audience" section holds the rules.

## When to write an ADR

- A parent RFC has settled the problem/direction and a specific technical decision now needs to
  be made and recorded before implementation.
- The decision is concrete enough to implement from.
- If a decision would genuinely serve more than one RFC, list all of them as parents rather than
  picking one arbitrarily and mentioning the rest only in prose — an unlisted relationship doesn't
  show up in the index tables.

## Format

Created/updated via `/adr-create`. File: `design/adr/ADR-NNN-short-title.md`, `NNN` zero-padded.

**Front-matter** (YAML):

| Field | Description |
|---|---|
| `id` | `ADR-NNN` |
| `title` | Short descriptive title |
| `status` | `proposed` \| `accepted` \| `rejected` \| `superseded` |
| `rfcs` | Parent RFC id(s), a list — **required, at least one**, set at creation, only ever grows (never hand-remove an entry) |
| `created` | ISO date (`YYYY-MM-DD`) |

**Required sections** (numbered `##`, renumber if one is omitted) — `status` lives only in
front-matter, no matching body section:

1. Context
2. Decision — split into numbered `###` subsections (`2.1`, `2.2`, ...) when the decision has
   more than one genuinely distinct facet (e.g. format, location, seeding plan)
3. Alternatives Considered
4. Consequences
5. Related — RFC link(s)

Use `/adr-review` before moving a draft to `accepted`.

## Index

| ADR | Title | Status | RFCs |
|---|---|---|---|
| [ADR-001](ADR-001-load-identity-and-merge.md) | Load Identity and Merge Semantics | accepted (amended 2026-11-08) | RFC-001, RFC-003 |
| [ADR-002](ADR-002-export-identity.md) | Export Identity | accepted (amended 2026-11-08) | RFC-001, RFC-003 |
| [ADR-003](ADR-003-file-kind-and-record.md) | File Kind and the Top-Level Record | accepted (amended 2026-11-08) | RFC-001, RFC-003 |
| [ADR-004](ADR-004-preserved-identity.md) | Preserved Identity | accepted | RFC-001, RFC-003 |
| [ADR-005](ADR-005-graph-metadata.md) | Graph Metadata and Meta Nodes | proposed | RFC-001, RFC-003 |
| [ADR-006](ADR-006-property-value-mapping.md) | Property Value Mapping | accepted | RFC-002 |
| [ADR-007](ADR-007-parser-path.md) | Parsing Through the tree-sitter-gram Syntax Tree | accepted | RFC-001, RFC-002 |
| [ADR-008](ADR-008-relationship-forms.md) | Relationship Types and Arrow Forms | accepted | RFC-001 |
| [ADR-009](ADR-009-command-line.md) | The relate-gram Command | proposed | RFC-001 |
