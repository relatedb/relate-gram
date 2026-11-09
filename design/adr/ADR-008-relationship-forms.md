---
id: ADR-008
title: Relationship Types and Arrow Forms
status: proposed
rfcs: [RFC-001]
created: 2026-11-09
---

# ADR-008: Relationship Types and Arrow Forms

## 1. Context

A Cypher relationship has exactly one type and exactly one direction: it runs from a start node to an end node. Gram is looser. A relationship can be written with no type, as `(a)-->(b)`, or with several labels, as `(a)-[:KNOWS:LIKES]->(b)`. It can also be written with no direction, as `(a)-[:KNOWS]-(b)`, or with two, as `(a)<-[:KNOWS]->(b)`. In the tree-sitter-gram syntax tree the arrow is a `right_arrow`, `left_arrow`, `undirected_arrow` or `bidirectional_arrow` node, and each of these comes in three families that differ by their line characters: thin (`-`), fat (`=`) and wavy (`~`). The relationship's type is the label list written inside the brackets.

A loader that accepts these forms has to invent something: a type for a relationship that has none, a choice among several labels, or a direction for an arrow that has none. Invented data would make the loaded database differ from the file without notice, which Constitution II forbids. In the gram ecosystem the fat and wavy families are reserved for schema and ontology files (RFC-001 §5.1), so they are not data arrows. The decision for the remaining cases was left open (RFC-001 question 7.8) and ADR-001 §2.4 refers to it for untyped relationships.

## 2. Decision

### 2.1 Accepted arrows

The loader accepts the thin right arrow (`-[…]->` and `-->`) and the thin left arrow (`<-[…]-` and `<--`). A right arrow `(a)-[r]->(b)` makes `a` the start node and `b` the end node; a left arrow `(a)<-[r]-(b)` makes `b` the start node and `a` the end node. In a path each arrow is read on its own, so `(a)-[:R]->(b)<-[:S]-(c)` is a relationship from `a` to `b` and one from `c` to `b`. ADR-001 §2.4 compares a repeated relationship's endpoints by start and end, so the written direction of a repeat does not matter.

### 2.2 Rejected arrows

Each of these is a load error that names the construct, reports the position of the arrow and says what to write instead:

- an undirected arrow (`-[:KNOWS]-`, `--`): "write a right or left arrow";
- a bidirectional arrow (`<-[:KNOWS]->`, `<-->`): "write a right or left arrow";
- a fat arrow (`==>`, `=[:KNOWS]=>`) or a wavy arrow (`~~>`, `~[:KNOWS]~>`), in any direction: "fat and wavy arrows are not data relationships; use a thin arrow" (RFC-001 §5.1).

The syntax tree has one node kind for each direction and does not distinguish the three families, so the loader reads the family from the first character of the arrow's text (`-`, `=` or `~`).

### 2.3 Relationship type

A relationship appearance that carries a record or labels is a definition (ADR-001 §2.4). A definition MUST have exactly one label, and that label is the relationship's type. These are load errors that report the position of the relationship:

- a definition with no label (`-[{since: 1843}]->`, `-[r {since: 1843}]->`): "add a type";
- a definition with two or more labels (`-[:A:B]->`, `-[r:A:B]->`): the message lists the labels and says a relationship has exactly one type;
- a bare arrow with no brackets (`-->`): it is an anonymous relationship with no type, so "add a type".

An identified appearance with neither labels nor record (`-[r]->`) is a reference, and ADR-001 §2.4 governs it.

Type text follows the rules for labels: a symbol or a quoted name, kept as written with the quotes removed.

### 2.4 Export

The exporter writes every relationship as a right arrow from its start node to its end node with its type, as ADR-002 §2.5 describes. It writes no other arrow form. A type is written as a bare symbol if it matches `[A-Za-z_][0-9A-Za-z_.@-]*`, and otherwise as a backtick-quoted name with the escapes ADR-002 §2.6 gives for identifiers. Node labels are written the same way (ADR-002 §2.5 carries the matching rule).

## 3. Alternatives Considered

3.1. **Load a relationship with no type under a fixed default type, such as `RELATED`.** Rejected: the file says nothing about the type, so the database would hold a value the author never wrote, and queries by type would match unrelated relationships.

3.2. **Load an untyped relationship under a type the caller names in a setting.** Rejected for the first phase: it adds a setting whose only use is to avoid writing a type in the file, and the same file would load differently under different settings.

3.3. **Load a bidirectional arrow as two relationships, one in each direction.** Rejected: the file wrote one relationship, so the database would hold two, with two identities where the file has one, and an export would not reproduce the arrow.

3.4. **Load an undirected arrow as a relationship in its written left-to-right order.** Rejected: the direction would come from the layout of the text, and the author did not intend it. The same graph written `(b)--(a)` would load differently.

3.5. **Take the first of several labels as the type and ignore the rest, or store the rest as a property.** Rejected: dropping a stated label is a silent drop, and moving it to a property creates data the file did not write.

3.6. **A setting that accepts an undirected arrow as a relationship in its written order.** Rejected: the same file would load to different graphs under different settings, as in 3.2 and 3.4.

3.7. **Accept fat and wavy arrows as thin arrows.** Rejected: pattern-rs reserves them for schema intent (RFC-001 §5.1), so accepting them would erase that meaning.

## 4. Consequences

- Files written for pattern matching, where `--` and `-->` are common, do not load until each relationship has a type and a direction. The error says what to write, and a file that loads is unambiguous.
- Gram that describes a symmetric relationship has to write it in one direction. A Cypher query can still match it in both.
- A relationship needs a type on its defining line, so the shorthand `(a)-->(b)` is not available for data files.
- Relationship identifiers that are only referenced stay an error (ADR-001 §2.4).
- The spec's examples and tests must use typed, directed relationships.
- A relationship written with two or more labels has no help beyond the error message; the author chooses the type and moves the rest to properties.
- ADR-002 gains a rule for writing labels (follow-up made with this ADR).
- Open: whether a file kind for schemas or ontologies reads the fat and wavy arrows with their own meaning (RFC-001 §5.1).

## 5. Related

- RFCs: RFC-001. Resolves its question 7.8: undirected and bidirectional thin arrows and untyped relationships are errors.
- ADR-001 §2.4 defines definitions and references of relationships; ADR-002 §2.5 the export layout; ADR-007 reads the arrow nodes.
