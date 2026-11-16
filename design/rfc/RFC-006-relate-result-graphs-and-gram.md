---
id: RFC-006
title: Relate Result Graphs and Gram
status: draft
created: 2026-11-13
adrs: []
---

# RFC-006: Relate Result Graphs and Gram

## 1. Summary

A property graph is a set of nodes and relationships, each carrying properties; nodes have labels and relationships have one type. Programs that handle graphs outside a database (a web page drawing a query result, a script transforming an export, a test comparing two graphs) have no common in-memory type for one. They use whatever their driver or visualization library returned, and Relate's own bindings return graph values that cannot be written to a file or read back.

Gram is a plain-text notation for graphs, and its strict subset (nodes, relationships, paths, scalar and homogeneous-array property values) is exactly a property graph. That subset can play for graphs the role JSON plays for trees: a data format with a text form that people can read and diff, and a native in-memory value in every language. What is missing is the join between that format and Relate: a property-graph model defined by the subset, conversions between it and the graphs that Relate query results carry, and gram as a built-in Relate format.

Reading and writing gram without a database, in every language and in a browser, is a separate problem: the gram ecosystem is designing a shared reader for it (RFC-005 5.5). This RFC consumes that reader. What remains is specific to Relate: the model as a profile of what the reader produces, the conversions with result graphs, identity, values the subset cannot express, and the built-in format. The exact shape of the model and its packaging are left to later decisions.

## 2. Problem / Motivation

**Every consumer of a graph result re-implements the same extraction.** Relate's Python binding returns a de-duplicated `ResultGraph` from `cursor.graph()` (Relate ADR-0214), and its JavaScript client has its own `ResultGraph` class in `clients/javascript/graph.js`. Both are two lists, nodes and relationships, with each entity appearing once. The notebook and visualization code that preceded them each carried an ad-hoc walk over result rows for the same purpose. The types exist in two languages, are not specified together, and cannot be written to a file or read back.

**There is no text form for a result graph that tools can exchange.** The WebAssembly build can render a result holding nodes as gram text, but nothing reads gram back (RFC-001 section 2, as of 2026-11-13). `relate-gram` loads and exports gram, but only through a database: to turn a gram file into a graph value, a program must start a Relate database, load the file, and query it back. Its library links `relate` (see its `Cargo.toml`), so it cannot run where `relate` does not, and the parser it uses is a C library (ADR-007) that needs a C standard library, which WebAssembly targets lack. A page that shows a query result, lets someone save it as a gram file, and reopens it later therefore cannot be built in a browser today. SPIKE-004 showed the parser itself can be built for WebAssembly with a pure-Rust runtime (about 170 KB, identical node counts to the native build on 214 inputs, about 5.6 times slower on a 5 MB file; measured on `tree-sitter-gram` 0.6.2, a browser not tested), so the obstacle is the missing model and the missing reader around it, not the parser.

**The strict subset is already specified, and has no standalone form.** The accepted rules (ADR-001 to ADR-008) say which gram is a property graph and which is rejected with a located error. Those rules are implemented as a loader that writes to a database. The same rules, applied to a value in memory, would define the data model, but the code that checks them is welded to the database write.

RFC-001 deferred language bindings and client-side result structures as longer-term goals (its goal 3.9, its non-goal 4.6). Phase 1 shipped, so those goals are next.

## 3. Goals

3.1. Define one in-memory property-graph model equal to the strict gram subset: nodes with labels and properties, relationships with one type, a start, an end and properties, property values that are scalars or homogeneous arrays of scalars, and an optional string identifier per element. A value of the model can always be written as valid gram.

3.2. Apply the strict subset's rules when gram text is read into the model, with the same located diagnostics as the loader gives.

3.3. Define how the model converts to and from the graph in a Relate query result, so a `.graph()` result can be written as gram text and a parsed graph can be handed to code that expects a result graph. State what is lost or reported when a result holds values the subset cannot express.

3.4. Make the model and the conversions usable without a Relate database, including in a browser to the extent the shared reader is (RFC-005 7.2).

3.5. Give the model no dependency on Relate, so that Relate can depend on it and offer gram as a built-in import and export format beside CSV and TOON (two of Relate's result output formats) without a dependency cycle.

## 4. Non-Goals

4.1. Parsing, serializing and packaging gram for each language. The shared reader and the strategies of RFC-005 provide them. An earlier draft of this RFC proposed a parser, a serializer and per-language packages; those goals moved there.

4.2. Any serialization of the model other than gram text. Gram already is the data format; a second one would duplicate it.

4.3. Query, traversal, pattern matching or algorithms over the model. It is a data structure, not a graph engine.

4.4. Gram constructs outside the strict subset (annotations, subject patterns, ranges, measurements, nested maps). The subset's boundary stays where RFC-001 put it.

4.5. Changing Relate's result types. The model converts to and from them and does not replace them.

4.6. Named graphs and graph metadata (RFC-003 and ADR-005 phases 2 and 3). The model starts as one graph with no named graphs, which is what the first phase of the project loads.

## 5. Proposed Approach (high-level)

### 5.1 The model is a profile of the reader's output

The model would hold exactly what the subset allows and nothing else. Anything that fails a rule of the subset (a nested map as a value, a relationship with two types) could not be built, so a value of the model could always be serialized to valid gram. The value rules are the ones RFC-002 and ADR-006 already set. The model holds only storable values, the types a database keeps as a property value (integer, float, boolean, string, a list of one of those, and the typed values that gram tags can name). Runtime values, which a query can return but a database cannot store as a property (a map, null, a node, a relationship, a path), are outside the model. The checks that now run during a load would become checks of building a graph, so parse errors and construction errors share one set of diagnostic codes.

If the shared reader produces a flat table of elements keyed by identity (a frame, RFC-005 5.5), the model is that table restricted to nodes and relationships under the subset's rules. If it does not, the model is defined over whatever the reader does produce. Either way the rules of the subset are a policy on top of the reader, as relate-gram's strictness is today (SPIKE-007).

### 5.2 Parse and serialize are the whole interface to text

Gram text goes in and a graph comes out, and the reverse, as `JSON.parse` and `JSON.stringify` do, using the shared reader for the reading half. A graph in a script is an ordinary value: it can be built by hand, compared, filtered by ordinary code, and written to a file. Ordering of elements in the text form follows the existing export rules, so serializing the same graph twice gives the same text.

### 5.3 Conversion with Relate results

Relate's result graph has the same shape. Its properties are stored values, so they are already storable, but they cover types the subset cannot yet write as gram (typed values without a tag, empty lists), and its keys and identifiers follow the engine's rules, not the subset's. Conversion to the model would report each value outside the subset, which includes every runtime value (a result column holding a map is legitimate Cypher but is not property data), and conversion from the model would fill the identity fields in the way the loader stores them today (the string identifier kept as the `@id` property, the reserved property that holds a gram identifier on a loaded element). The model and the result graph would stay separate types joined by this mapping. Making them one type would impose the subset's limits on query results that legitimately hold dates, points and maps.

### 5.4 The model does not depend on Relate

The model, the rules and the conversions' neutral half would not depend on Relate. The load and export that write to a database stay as a layer on top, and Relate's bindings own the conversions that touch their own result types. Relate can then adopt the model as a dependency for a built-in format without the model knowing about Relate. This changes the project's stated purpose (an extension that links Relate) and the constitution's Rust constraints (a crate that depends on `relate`), so an ADR for the split would also amend both, as ADR-007 amended the parser constraint. Packaging for each language follows RFC-005.

### 5.5 Gram as a Relate format

Once the model and the reader exist, Relate can offer gram beside its other formats: as an output mode for results and as an import and export option. Until then, `relate run` can host `relate-gram` as an extension. Whether the built-in format replaces or sits beside the `relate-gram` command is decided with Relate's maintainers.

## 6. Alternatives Considered

6.1. **Make the model identical to Relate's result graph.** Rejected: the result graph carries values the subset rejects, and its identifiers are engine identifiers; one type would have to give up one of them.

6.2. **Define a JSON form of the model for client use.** Rejected: gram text is the serialization, and every target language can read and produce it through the shared reader.

6.3. **Keep gram behind a database, and let clients load a file into an embedded Relate to get a graph.** Rejected: it ties every consumer to a database engine and its size, to read a file.

6.4. **Use an existing graph library's types in each language.** Rejected: there is no common type across languages, and none is defined by the strict subset or has gram as its text form.

6.5. **Give this model its own parser and serializer.** Rejected: the shared reader is the place for reading gram, and a second reader would repeat the divergence found among the existing ones (SPIKE-006).

6.6. **Add `parse` and `serialize` to Relate's existing result-graph types instead of defining a model.** Rejected: those types hold values the subset rejects and engine-owned identifiers, so a parsed gram file would become a result graph with fields that have no meaning. They are also defined inside Relate's bindings, so code that does not use a Relate binding could not use them.

6.7. **Use an existing graph interchange format (GraphML, for example) as the file format.** Rejected: none of them is a data model with a text form that people read and diff and that maps directly onto Cypher's property graph the way the strict subset does.

## 7. Open Questions

7.1. What identity does a model element carry: only the gram identifier, or also an engine identifier when it came from a result? How do those map to `@id` and to `elementId()` in both directions?

7.2. Does the model include the file's top-level record, or only nodes and relationships?

7.3. Is the model immutable, mutable, or built through a builder? What minimal lookups does a data structure need (an element by identifier, a relationship's endpoints) without becoming a query layer?

7.4. When a result contains values outside the subset, does conversion fail on the first, report all and omit the element, or offer both through an option?

7.5. How does the model extend to named graphs and graph metadata in the second phase (RFC-003)?

7.6. Does the model keep paths as written, so that serializing reproduces them, or only nodes and relationships? What does a round trip of a file that wrote its graph as paths produce? The answer depends on what the shared reader retains of arrow style and path structure.

7.7. Does a built-in Relate format replace the `relate-gram` command, or sit beside it, and who owns the change in Relate?

7.8. Is the model a profile of the shared reader's output (5.1), or a separate type converted from it?

Deferred to the ADRs: where the model lives and how it is versioned relative to `relate-gram` and Relate's releases; whether bindings expose native objects or wrap a handle owned by the model's implementation.

## 8. ADRs

_(populated automatically as `/adr-create` links ADRs to this RFC)_
