---
id: RFC-002
title: Property Value Mapping Between Gram and Cypher
status: review
created: 2026-11-06
adrs: []
---

# RFC-002: Property Value Mapping Between Gram and Cypher

## 1. Summary

Gram and Cypher both attach key-value properties to nodes and relationships, but their value vocabularies differ. Gram has strings in several quoting styles, integers in decimal, hexadecimal and octal spellings, decimals, booleans, bare symbols (unquoted names such as `active`), ranges, measurements, tagged strings (a string carrying a tag such as a format name), arrays of scalars and nested maps. Cypher stores a fixed set of types, and its lists hold values of one type. Loading gram into a Cypher database and exporting back out requires a rule for every value that crosses the boundary.

The proposed direction for that rule has two phases. The first is strict: values are accepted on load only if they become a Cypher type, and exported Cypher values are written as the gram values that correspond to them, so that a round trip preserves types. Alternative spellings of integers and strings are accepted and converted. Tagged strings are accepted too: a tag that names a well-known Cypher type (temporal or spatial) produces a value of that type, and any other tag is dropped, leaving a plain string. A later phase adds an advanced mapping in which metadata about properties is recorded alongside the graph, so that information gram can express but Cypher cannot store, such as a dropped tag, survives the trip. The motivating example is a string property known to contain markdown, which could be written back as a tagged string.

The mapping tables, edge cases and metadata format are left to ADRs.

## 2. Problem / Motivation

**Value types cannot be left implicit.** The gram grammar (its number, range, measurement and tagged-string rules, in tree-sitter-gram) accepts, among others, `{count: 0x1F}`, `{status: active}`, `{span: 1..5}`, `{weight: 5kg}` and ``{body: md`# Title`}``. A Cypher database stores none of these as they are written. A loader that guesses (storing `active` as a string, or a measurement as text) produces a database whose values differ from the file's in ways the user cannot see, and an exporter that guesses back cannot reproduce the original text.

**Type fidelity is what makes round trips trustworthy.** If an integer loads as a float, or a list of mixed values is flattened to strings, a graph exported and reloaded no longer matches the one before, and queries over it give different answers. The first phase therefore commits to preserving the types Cypher can store, in both directions.

**Some information has no Cypher home.** A property such as `body` may hold markdown, and gram's tagged strings can say so. Cypher stores a plain string and loses the tag. Keeping that information is useful for tools that render or process the property, and it cannot live in the value itself without changing what queries see. It needs somewhere else to be recorded.

## 3. Goals

3.1. Define, for every value kind gram can express, whether it is accepted on load and what Cypher value it becomes.

3.2. Define, for every Cypher property value Relate can store, the gram value it is written as on export.

3.3. In the first phase, preserve Cypher types exactly through a round trip, convert alternative spellings of integers and strings to those types, and reject any other value that does not become a Cypher type with an error that locates it.

3.4. Provide a direction for an advanced mapping that records metadata about properties, so that information such as "this string is markdown" survives a round trip without changing the stored value.

3.5. Make the mapping a single documented rule that the loader, the exporter and any later language bindings share.

## 4. Non-Goals

4.1. Storing gram values in Cypher in a form that queries cannot use naturally. A stored value is an ordinary Cypher value.

4.2. Extending Cypher's type system, or changing how Relate stores values.

4.3. Specifying the metadata format in this RFC. Direction only.

4.4. Supporting nested maps, ranges, measurements or other values that cannot become a Cypher type in the first phase.

4.5. Preserving a tag that does not name a Cypher type. In the first phase it is dropped.

## 5. Proposed Approach (high-level)

### 5.1 Strict first phase

A gram value is accepted if it becomes a type Cypher stores. The accepted set is strings, integers, decimals (as floats), booleans, arrays whose elements are all of one such type, and tagged strings (5.2). Alternative spellings are accepted and converted: hexadecimal and octal integers become integers; bare symbols used as values (unquoted names such as `active`), and single-quoted, backticked and fenced strings, become strings. Whatever is accepted is stored as an ordinary Cypher value. Gram constructs that cannot become a Cypher type (ranges, measurements, nested maps) are load errors.

Export is the reverse: each stored Cypher value is written in the gram form that reads back as the same type. Where gram offers several spellings, export picks one canonical spelling. Conversion on load is therefore not reversible spelling-for-spelling: `0x1F` reads back as `31`.

### 5.2 Tagged strings and Cypher's other types

Cypher stores types such as temporal values and spatial points that gram has no literal for. Gram's tagged string is the mechanism for them: a tag names the Cypher type and the string holds its textual form in a standard convention, such as ISO-8601 for temporal values and WKT for spatial ones.

- A tag naming a well-known Cypher type (the vocabulary is left to an ADR) produces a value of that type on load, and a value of that type is written as a tagged string on export. This is part of the strict phase, because the tag only names a type the stored value then has.
- Any other tag is accepted and dropped: the content loads as an ordinary string. The tag is lost in the first phase, which is the gap the advanced mapping closes.

### 5.3 Advanced mapping with property metadata

In the later phase, the database holds metadata about properties, kept apart from the values themselves. Metadata says things like "the `body` property of `Document` nodes holds markdown". On load, a tagged string with a tag that is not a Cypher type stores its content as an ordinary string and records the tag in the metadata. On export, the metadata is consulted and the property is written as a tagged string again. Queries see only the plain string.

The meta node, which holds a gram file's top-level record (RFC-001), is the candidate place to record this metadata, because it already represents information about the graph rather than in it. Whether the meta node is the right home, and at what granularity metadata attaches (a property key across a label, a single element, a whole named graph), is open.

### 5.4 Cross-cutting concerns

The mapping applies equally to node properties, relationship properties and the meta node's own properties. Errors are reported with a position in the source file. The rule is the same in every named graph.

## 6. Alternatives Considered

6.1. **Store every gram value as a string.** Rejected: it discards types, and arithmetic, comparison and sorting in Cypher then give different answers from the source data.

6.2. **Encode extra information in the stored value (for example by embedding the tag in the string).** Rejected: queries would see a value that differs from the data, and every consumer would have to strip the encoding.

6.3. **Accept every gram value, storing what Cypher cannot hold as a string.** Rejected: a range, measurement or map rendered as text makes a loaded graph differ from its source without notice, so these fail loudly. Tagged strings differ because their content already is a string.

6.4. **Reject tagged strings whose tag does not name a Cypher type.** Rejected: the content is an ordinary string and loads correctly, and rejecting would make many existing gram files unloadable for a loss that the advanced mapping can later recover.

6.5. **Record dropped tags as companion properties on the element.** Rejected: it adds properties that queries and exports see, filling the data model with bookkeeping. Metadata kept apart from the data avoids this.

6.6. **Add the advanced mapping at the start.** Deferred: the strict mapping is needed either way and is independently useful, and the metadata design depends on decisions about the meta node that are not yet made.

## 7. Open Questions

7.1. How are numeric values bounded: the width and range of integers, floats, and out-of-range or non-finite values?

7.2. What is the canonical gram spelling for each Cypher type on export, including quoting and escaping of strings?

7.3. Which of Cypher's non-basic types (temporal values, points, vectors, byte arrays) does Relate store as properties? What is the tag vocabulary for the well-known ones, and which textual conventions apply to each (for example, which ISO-8601 forms for temporal values, which WKT dialect for spatial ones)?

7.4. What happens when a tagged string's tag names a well-known type but its content does not parse as that type: a load error, or a fallback to a plain string?

7.5. How are lists mapped, given that Cypher lists hold one type and gram arrays hold scalars? What happens to an empty list, whose element type cannot be inferred from its text, and to a list of mixed types? Do lists of tagged values (for example temporal values) work?

7.6. Is dropping an unrecognized tag reported by default (as a warning), and can the user make it an error or silence it?

7.7. Where is property metadata recorded, and at what granularity? Candidates are the meta node (keyed by label and property key), per-element annotations, or a separate structure. How does metadata interact with named graphs, where each graph may carry its own?

7.8. What happens when metadata and data disagree: a property marked as markdown that holds a non-string, or two loads of the same named graph recording different tags for one property?

7.9. Should the strict phase and the advanced phase be one switchable mode of a single mapping, or two mappings sharing a common core?

7.10. How are property keys quoted and escaped? What happens to duplicate keys in one map, and how does export treat Cypher values for which no gram form exists (for example vectors or byte arrays), given that Cypher has no null property and gram has no null?

## 8. ADRs

_(populated automatically as `/adr-create` links ADRs to this RFC)_
