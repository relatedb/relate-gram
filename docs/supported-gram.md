# What is supported, and why

`relate-gram` loads a strict subset of [gram](https://github.com/gram-data/gram-spec): the part that
maps directly to Cypher's property graph model. A file that uses anything else is rejected with an
error that names the construct, shows where it is, and says what to write instead. Nothing is
approximated or dropped silently; the two reported exceptions are a tag on a tagged string and the
top-level record's properties, which are listed as warnings.

Run `relate-gram check FILE` to find every problem in a file at once.

## Supported

- **Nodes** `(a:Person:Admin {name: "Ada"})`: any number of labels and properties.
- **Relationships** `(a)-[k:KNOWS {since: 1840}]->(b)` and `(a)<-[:KNOWS]-(b)`: exactly one type, and a
  direction.
- **Paths** `(a)-[:R]->(b)-[:S]->(c)`: shorthand for the nodes and relationships they spell out.
- **Identifiers** `a`, `42`, `` `my id` ``: name an element so other parts of the file can refer to it.
  Each is stored as the property `@id`, and a later load matches by it.
- **Property values**: strings (double-quoted, single-quoted, backticked or fenced), integers (decimal
  or `0x` hexadecimal, 64 bits), decimals, `true` and `false`, bare words (`active` is the string
  `"active"`), and arrays of one kind (`[1, 2, 3]`).
- **The top-level record** `{kind: "property-graph", name: "roster"}`: checked, and its properties are
  reported as not stored.
- **Comments**, `//` and `/** ... */`: ignored.

## Rejected, and what to write instead

| You wrote | Error code | Write instead |
|---|---|---|
| `(a)-->(b)`, `(a)-[{since: 1}]->(b)` | `rel.no-type` | Give the relationship a type: `(a)-[:KNOWS]->(b)` |
| `(a)-[:A:B]->(b)` | `rel.many-labels` | A relationship has exactly one type; keep one and move the other to a property |
| `(a)--(b)`, `(a)-[:T]-(b)` | `rel.undirected` | Pick a direction: `(a)-[:T]->(b)` |
| `(a)<-->(b)`, `(a)<-[:T]->(b)` | `rel.bidirectional` | Pick a direction, or write two relationships |
| `(a)==>(b)`, `(a)~~>(b)` | `rel.arrow-family` | Use a thin arrow: `(a)-[:T]->(b)` |
| `[x \| (a), (b)]` | `construct.subject-pattern` | Write the nodes and relationships separately |
| `@desc("x") (a)` | `construct.annotation` | Remove the annotation; put the information in a property |
| `(a::Label)`, `{k:: 1}` | `construct.double-colon` | Use a single colon |
| `{r: 1..5}` | `value.range` | Store the bounds as two properties (`low: 1, high: 5`) |
| `{h: 168cm}` | `value.measurement` | Store a number and a unit (`height: 168, unit: "cm"`) |
| `{m: {k: 1}}` | `value.map` | Flatten the entries into separate properties |
| `{o: 010}` | `value.octal` | Write the value in decimal (`8`) or hexadecimal (`0x8`) |
| `{n: 9223372036854775808}` | `value.integer-range` | Integers must fit in 64 bits; use a string for larger numbers |
| `{xs: [1, "a"]}`, `{xs: [1, 2.5]}` | `value.array-mixed` | An array holds one kind of value; write every element as that kind (`[1.0, 2.5]`) |
| `{k: 1, k: 2}` | `key.duplicate` | Write the key once |
| `{"": 1}` | `key.empty` | Give the property a name |
| `{"@k": 1}` | `key.reserved` | Keys that start with `@` are written by the loader; rename it |
| `(a:L)` then `(a:M)` | `id.duplicate-definition` | Define an element once, then refer to it by its identifier: `(a)` |
| `(a)-[a:R]->(b)` | `id.kind-conflict` | An identifier names a node or a relationship, not both |
| `(a)-[r:R]->(b)` then `(a)-[r]->(c)` | `id.endpoints-differ` | A repeated relationship identifier must connect the same two nodes |
| `(a)-[r]->(b)` and `r` is never defined | `id.rel-never-defined` | Define it once with a type: `-[r:R]->` |
| `{kind: "schema"}` | `file.kind` | Only `property-graph` files are loaded |

These are the codes `check`, `load` and `normalize` report. A code never changes meaning; the full list,
with the warnings, is in
[`specs/001-phase1-load-export/contracts/diagnostics.md`](../specs/001-phase1-load-export/contracts/diagnostics.md).

## Why so strict

A loaded database that quietly differs from its file cannot be trusted. Each rule above exists because
the alternative is to invent something the file does not say: a default relationship type, a direction
for an undirected arrow, a number from an octal literal. The reasons are in the ADRs, in
[`design/adr/`](../design/adr/) (ADR-001, 006, 007 and 008).

If a rejection is in your way, the error message is the shortest path to a file that loads. A command
that repairs files automatically is planned
([notes](../design/notes/2026-11-09-fixing-rejected-files.md)).
