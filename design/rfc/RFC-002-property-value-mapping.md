---
id: RFC-002
title: Property Value Mapping Between Gram and Cypher
status: draft
created: 2026-11-06
adrs: []
---

# RFC-002: Property Value Mapping Between Gram and Cypher

## 1. Summary

Gram and Cypher both attach key-value properties to nodes and relationships, but their value vocabularies differ. Gram has strings in several quoting styles, integers in decimal, hexadecimal and octal spellings, decimals, booleans, bare symbols, ranges, measurements, tagged strings (a string carrying a tag such as a format name), arrays of scalars and nested maps. Cypher stores a fixed set of types, and its lists hold values of one type. Loading gram into a Cypher database and exporting back out requires a rule for every value that crosses the boundary.

This RFC proposes the direction for that rule. The first phase is strict: only values that correspond to a Cypher type are accepted on load, and exported Cypher values are written as the gram values that correspond to them, so that a round trip preserves types. A later phase adds an advanced mapping in which metadata about properties is recorded alongside the graph, so that information gram can express but Cypher cannot store survives the trip. The motivating example is a string property known to contain markdown, which could be written back as a tagged string.

The RFC fixes the direction and the open questions. The mapping tables, edge cases and metadata format belong in ADRs.

## 2. Problem / Motivation

**Value types cannot be left implicit.** The gram grammar accepts, among others, `{count: 0x1F}`, `{status: active}`, `{span: 1..5}`, `{weight: 5kg}` and ``{body: md`# Title`}``. A Cypher database stores none of these as they are written. A loader that guesses (storing `active` as a string, or a measurement as text) produces a database whose values differ from the file's in ways the user cannot see, and an exporter that guesses back cannot reproduce the original text.

**Type fidelity is what makes round trips trustworthy.** If an integer loads as a float, or a list of mixed values is flattened to strings, a graph exported and reloaded no longer matches the one before, and queries over it give different answers. The first phase therefore commits to preserving the types Cypher can store, in both directions.

**Some information has no Cypher home.** A property such as `body` may hold markdown, and gram's tagged strings can say so. Cypher stores a plain string and loses the tag. Keeping that information is useful for tools that render or process the property, and it cannot live in the value itself without changing what queries see. It needs somewhere else to be recorded.

## 3. Goals

3.1. Define, for every value kind gram can express, whether it is accepted on load and what Cypher value it becomes.

3.2. Define, for every Cypher property value Relate can store, the gram value it is written as on export.

3.3. In the first phase, preserve Cypher types exactly through a round trip, and reject any value that does not correspond to a Cypher type with an error that locates it.

3.4. Provide a direction for an advanced mapping that records metadata about properties, so that information such as "this string is markdown" survives a round trip without changing the stored value.

3.5. Make the mapping a single documented rule that the loader, the exporter and any later language bindings share.

## 4. Non-Goals

4.1. Storing gram values in Cypher in a form that queries cannot use naturally. A stored value is an ordinary Cypher value.

4.2. Extending Cypher's type system, or changing how Relate stores values.

4.3. Specifying the metadata format in this RFC. Direction only.

4.4. Supporting nested maps or values with no Cypher type in the first phase.

## 5. Proposed Approach (high-level)

### 5.1 Strict first phase

A gram value is accepted if it corresponds to a type Cypher stores. The accepted set starts from strings, integers, decimals (as floats), booleans, and arrays whose elements are all of one such type. Some gram spellings (hexadecimal and octal integers, bare symbols used as values, alternative string quoting) express a value that could be stored as one of those types, and whether each is accepted is left open below. Whatever is accepted is stored as a plain Cypher value. Gram constructs with no counterpart (ranges, measurements, nested maps, and tagged strings in the first phase) are load errors.

Export is the reverse: each stored Cypher value is written in the gram form that reads back as the same type. Where gram offers several spellings, export picks one canonical spelling.

### 5.2 Cypher types beyond the basics

Cypher also stores types such as temporal values and spatial points, which gram has no literal for. Gram's tagged string is a possible carrier: a tag naming a Cypher type and a string holding its textual form. Whether and how to use it is an open question, because it sits between the strict first phase and the advanced mapping. The deciding factor is whether the tag only names a Cypher type that the stored value then has (a strict mapping), or records something about the value that the stored value does not have (an advanced one).

### 5.3 Advanced mapping with property metadata

In the later phase, the database holds metadata about properties, kept apart from the values themselves. Metadata says things like "the `body` property of `Document` nodes holds markdown". On load, a tagged string stores its content as an ordinary string and records the tag in the metadata. On export, the metadata is consulted and the property is written as a tagged string again. Queries see only the plain string.

The top-level record's meta node, introduced in RFC-001, is the candidate place to record this metadata, because it already represents information about the graph rather than in it. Whether the meta node is the right home, and at what granularity metadata attaches (a property key across a label, a single element, a whole namespace), is open.

### 5.4 Cross-cutting concerns

The mapping applies equally to node properties, relationship properties and the meta node's own properties. Errors are reported with a position in the source file. The rule is the same in every namespace.

## 6. Alternatives Considered

6.1. **Store every gram value as a string.** Rejected: it discards types, and arithmetic, comparison and sorting in Cypher then give different answers from the source data.

6.2. **Encode extra information in the stored value (for example by embedding the tag in the string).** Rejected: queries would see a value that differs from the data, and every consumer would have to strip the encoding.

6.3. **Accept all gram values, storing the unmappable ones as strings.** Rejected for the first phase: a silent downgrade makes a loaded graph differ from its source without notice. The strict phase fails loudly instead.

6.4. **Add the advanced mapping at the start.** Deferred: the strict mapping is needed either way and is independently useful, and the metadata design depends on decisions about the meta node that are not yet made.

## 7. Open Questions

7.1. Which gram spellings of an accepted type are accepted on load: hexadecimal and octal integers, bare symbols as values, and single-quoted, backticked and fenced strings? Is each a spelling of a Cypher type, or a distinct construct to reject?

7.2. How are numeric ranges handled: the width and range of integers, floats, and out-of-range or non-finite values?

7.3. What is the canonical gram spelling for each Cypher type on export, including quoting and escaping of strings?

7.4. Which of Cypher's non-basic types (temporal values, points, vectors, byte arrays) does Relate store as properties, and should any be representable in gram through tagged strings in the first phase?

7.5. How are lists mapped, given that Cypher lists hold one type and gram arrays hold scalars? What happens to an empty list, whose element type cannot be inferred from its text, and to a list of mixed types?

7.6. What happens to a tagged string in the first phase: rejected, or accepted with the tag discarded (with a warning)? If discarded, how does the user opt in?

7.7. Where is property metadata recorded, and at what granularity? Candidates are the meta node (keyed by label and property key), per-element annotations, or a separate structure. How does metadata interact with namespaces, where each graph may carry its own?

7.8. What happens when metadata and data disagree: a property marked as markdown that holds a non-string, or two loads of the same namespace recording different tags for one property?

7.9. Which metadata beyond "this string has a format tag" is worth recording (for instance units for measurements), and which gram constructs would the advanced phase make acceptable?

7.10. Should the strict phase and the advanced phase be one switchable mode of a single mapping, or two mappings sharing a common core?

## 8. ADRs

_(populated automatically as `/adr-create` links ADRs to this RFC)_
