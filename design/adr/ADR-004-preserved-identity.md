---
id: ADR-004
title: Preserved Identity
status: accepted
rfcs: [RFC-001, RFC-003]
created: 2026-11-08
---

# ADR-004: Preserved Identity

## 1. Context

The project constitution states, in Principle III, that on load "gram identifiers MUST only resolve references within the file; they are not stored." Principle II lists one documented exception to its rule that nothing is dropped: an unrecognized tag on a tagged string. Both were written when a load was a one-shot import. Three things now argue against them:

- **Reloading an edited file.** A user loads a gram file, edits it, and loads it again, expecting the database to match the file. If identifiers are dropped, nothing says which elements the first load created, so the second load duplicates every element (RFC-003).
- **Round trips.** Principle IV promises that loading a file and exporting the result preserves the graph. An export that cannot write back the identifiers the file stated does not keep what the file said (ADR-002).
- **Identity is fundamental to gram.** Gram files name their elements, and the names are how other parts of the file refer to them. Systems that persist such names do it by an explicit step at the load boundary: Datomic resolves each local id to a permanent one, and RDF skolemization replaces a blank node's label with a stable identifier.

A second rule needs an exception. The top-level record of a gram file has nowhere to be stored in the first phase of RFC-003, so its properties are reported as not stored (ADR-003). That is a drop the constitution does not list.

The constitution says an amendment requires an ADR that describes the change and its rationale and that updates the document and its version.

## 2. Decision

### 2.1 Principle III

Principle III is replaced by the following.

> ### III. Identity Is Preserved
> On load, gram identifiers MUST resolve references within the file and MUST be kept on the element as the reserved property `@id`, or the element's identity MUST come from a property that the caller names. An element is defined at one site and referenced elsewhere by its identifier alone, and a second definition MUST be an error. The first appearance of an identifier fixes it as a node or a relationship, and a later conflicting use MUST be an error. On export, identity MUST be configurable and MUST have deterministic defaults. A missing or non-unique identifier source MUST be an error that lists the offending elements, never a silent fallback. An element that has no identifier in the file is stored without identity, and the loader MUST report how many such elements it loaded.
>
> Rationale: identity that is dropped makes a second load duplicate every element and leaves an export unable to restore what the file said. Explicit rules for where identity comes from keep two loads of one file in agreement.

### 2.2 Principle II

The sentence naming the one documented exception is replaced by: "The documented exceptions are an unrecognized tag on a tagged string, whose tag is dropped, and the properties of a file's top-level record while nothing stores them; the loader MUST report each."

### 2.3 Version

The change redefines Principle III and adds an exception to Principle II, so the constitution's version becomes 2.0.0 when this ADR is accepted, and its last-amended date is that day. Principles I, IV, V and VI are unchanged.

Accepting this ADR and editing the constitution are one change, made in one commit. The constitution's Sync Impact Report cites this ADR.

## 3. Alternatives Considered

3.1. **Keep Principle III and store nothing.** Rejected: it leaves every reload duplicating the first load and every export unable to write back the file's identifiers.

3.2. **Amend Principle III only for named graphs.** Rejected: identity is needed in the first phase, where there are no named graphs, and two rules for identity would be harder to state and to test.

3.3. **Make storing identity an option that defaults to off.** Rejected: the default would then duplicate on reload, which is the surprise the principle exists to prevent.

3.4. **Add a new principle for identity and leave III as it is.** Rejected: Principle III already covers identity, and two principles would contradict each other.

## 4. Consequences

- The constitution is updated to 2.0.0 when this ADR is accepted, and its Sync Impact Report records the change.
- ADR-001, ADR-002 and ADR-003 carry amendments that take effect on acceptance.
- Every identified element carries an `@id` property that queries show. A load that wants no stored identity has no option in the first phase; if throwaway fixtures prove to need one, it is added later.
- The reported drop of a top-level record's properties becomes a named exception to Principle II, so any further drop needs the same documentation.
- `CLAUDE.md` is updated to match when this ADR is accepted.

## 5. Related

- RFCs: RFC-001, RFC-003
