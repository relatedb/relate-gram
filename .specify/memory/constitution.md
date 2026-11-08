<!--
Sync Impact Report (ADR-004)
Version change: 1.0.0 -> 2.0.0 (MAJOR)
Modified principles: II (second documented exception: top-level record properties);
III "Identity Is Explicit" -> "Identity Is Preserved"
Added/removed sections: none
-->

# relate-gram Constitution

## Core Principles

### I. Design Before Code
Work proceeds RFC → ADR → Spec Kit specification → implementation. An accepted ADR is the
authority for a technical decision. A specification, plan or task MUST name the ADRs it
implements and MUST NOT restate them; where they differ, the ADR wins until it is amended.
Commits and pull requests MUST cite the ADR number they implement.

Rationale: decisions about identity, namespaces and value mapping are expensive to reverse, and
one authoritative record keeps specifications and code from drifting apart.

### II. Strict Subset, Loud Failures
The loader MUST accept only gram constructs that map directly to Cypher's property graph model.
Every other construct MUST be a load error that reports its position in the source file. Nothing
is dropped or approximated silently. The documented exceptions are an unrecognized tag on a
tagged string, whose tag is dropped, and the properties of a file's top-level record while nothing
stores them; the loader MUST report each.

Rationale: a loaded database that differs from its source without notice cannot be trusted.

### III. Identity Is Preserved
On load, gram identifiers MUST resolve references within the file and MUST be kept on the element
as the reserved property `@id`, or the element's identity MUST come from a property that the caller
names. An element is defined at one site and referenced elsewhere by its identifier alone, and a
second definition MUST be an error. The first appearance of an identifier fixes it as a node or a
relationship, and a later conflicting use MUST be an error. On export, identity MUST be
configurable and MUST have deterministic defaults. A missing or non-unique identifier source MUST
be an error that lists the offending elements, never a silent fallback. An element that has no
identifier in the file is stored without identity, and the loader MUST report how many such
elements it loaded.

Rationale: identity that is dropped makes a second load duplicate every element and leaves an
export unable to restore what the file said. Explicit rules for where identity comes from keep two
loads of one file in agreement.

### IV. Round-Trip Fidelity
Loading a file and exporting the result MUST preserve the graph and its property types. Exports of
the same database MUST be byte-stable and diffable. Parser conformance MUST be tested against the
tree-sitter-gram test corpus.

Rationale: interchange is only useful if what comes out is what went in.

### V. Public Interfaces Only
The crate MUST depend only on Relate's public library interface and its documented extension
mechanism, with no coupling to private engine internals. Anything needed beyond that is raised
with the Relate project as a request.

Rationale: the crate is developed out of tree and is intended to be brought into the Relate
project later without rework.

### VI. Test-First (NON-NEGOTIABLE)
Tests MUST be written and observed to fail before the implementation they cover. Every feature
MUST have contract tests for load and export, round-trip tests, and tests that assert the reported
position of each error. A change that makes any of these fail MUST NOT be merged.

Rationale: the behavior is defined by contracts and error reporting, which tests state precisely.

## Rust Constraints

- The crate MUST depend on `relate` and `gram-codec` (from pattern-rs) for the engine and the
  parser; any other parser requires an ADR.
- The Rust toolchain MUST be pinned in the repository and bumped deliberately.
- `unsafe` code MUST NOT be added without an ADR.
- Formatting and lint checks (`cargo fmt`, `cargo clippy`) MUST pass before merge.

## Development Workflow

- Design documents and code are authored on feature branches, never directly on the main branch.
- An RFC, ADR or spike MUST pass its review skill (`/rfc-review`, `/adr-review`) before its status
  moves toward `review` or `accepted`.
- Drafts are reviewed through a pull request before they merge.
- Every Spec Kit plan MUST include a check against this constitution.

## Governance

This constitution supersedes other project practices. An amendment requires an ADR describing the
change and its rationale, and MUST update this document and its version. Versions follow semantic
versioning: MAJOR for removing or redefining a principle, MINOR for adding a principle or section
or materially expanding guidance, PATCH for clarifications. Every plan and pull request review MUST
verify compliance with the principles above, and any deviation MUST be justified in writing.
Runtime guidance for agents lives in `CLAUDE.md`.

**Version**: 2.0.0 | **Ratified**: 2026-11-06 | **Last Amended**: 2026-11-08
