# RFCs — Requests for Comment

RFCs capture **WHAT** a problem is and **WHY** it matters, plus high-level candidate directions —
the Discover/Define half of the double-diamond design process described in the project's
[CLAUDE.md](../../CLAUDE.md). They deliberately exclude implementation detail; that lives in one
or more child [ADRs](../adr/README.md).

## Audience

Write for a working engineer who has just found this repository. Assume they are technically
fluent and comfortable with graph databases and the gram notation as general ideas, and assume
nothing about this project's specifics: not the meta node, not the named graphs, not any
sibling RFC or ADR. Name project artifacts (the meta node, the named graph) rather than
assuming them; one clause on first use is enough.

The failure mode to avoid is a document addressed to its own authors — one that argues with past
decisions, defends choices nobody has questioned, and cites sibling documents before naming its
subject. `/rfc-review` criterion 8 checks for this, and `rfc-create`'s "Voice and audience" section
holds the rules. Section 1 in particular has to teach before it locates: strip every `§`-reference
and RFC/ADR number out of it, and what remains must still explain the topic to a newcomer.

## When to write an RFC

- A new capability or subsystem is being considered and the problem/scope isn't settled yet.
- Multiple genuinely different high-level directions exist and the trade-off is worth recording.
- Before any `/adr-create` work — every ADR requires at least one parent
  RFC (an ADR MAY serve more than one RFC when it's genuinely shared infrastructure).

## Format

Created/updated via `/rfc-create`. File: `design/rfc/RFC-NNN-short-title.md`, `NNN` zero-padded.

**Front-matter** (YAML):

| Field | Description |
|---|---|
| `id` | `RFC-NNN` |
| `title` | Short descriptive title |
| `status` | `draft` \| `review` \| `accepted` \| `superseded` |
| `created` | ISO date (`YYYY-MM-DD`) |
| `adrs` | List of child ADR ids — maintained automatically by `/adr-create`, do not hand-edit |

**Required sections** (numbered `##`, renumber if one is omitted):

1. Summary
2. Problem / Motivation
3. Goals
4. Non-Goals
5. Proposed Approach (high-level) — split into numbered `###` subsections when more than one
   distinct axis or strategy is being explored
6. Alternatives Considered
7. Open Questions — each item individually numbered (`7.1.`, `7.2.`, ...), never a plain bullet
   list, so other sections, ADRs, and future RFCs can cite exactly which question they address
8. ADRs — list of child ADRs, kept in sync with the `adrs:` front-matter field

Optional appendices may follow as additional numbered sections (`## 9. Appendix: ...`, etc.) —
useful for a research-level comparative evaluation of candidate approaches from section 5 that's
too detailed for the main body but still belongs at the RFC's WHAT/WHY level (i.e. it compares
options, it doesn't design one).

Use `/rfc-review` before moving a draft to `review`/`accepted`.

## Index

| RFC | Title | Status | ADRs |
|---|---|---|---|
| [RFC-001](RFC-001-gram-support-for-relate.md) | Gram Graph Support for Relate | review | ADR-001, ADR-002, ADR-003, ADR-004, ADR-005, ADR-007, ADR-008, ADR-009 |
| [RFC-002](RFC-002-property-value-mapping.md) | Property Value Mapping Between Gram and Cypher | review | ADR-006, ADR-007, ADR-011 |
| [RFC-003](RFC-003-named-graphs-and-reloading.md) | Named Graphs and Reloading Changed Files | draft | ADR-001, ADR-002, ADR-003, ADR-004, ADR-005 |
| [RFC-004](RFC-004-fixing-rejected-gram-files.md) | Fixing Rejected Gram Files | review | ADR-010 |
