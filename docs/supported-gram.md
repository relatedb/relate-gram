# What is supported, and why

`relate-gram` loads a strict subset of [gram](https://github.com/gram-data/gram-spec): the part that
maps directly to Cypher's property graph model. A file that uses anything else is rejected with an
error that names the construct, shows where it is, and says what to write instead. Nothing is
approximated or dropped silently; the two reported exceptions are an unrecognized tag on a tagged string
and the top-level record's properties, which are listed as warnings.

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
- **Typed values**: a tagged string whose tag is one of eight names becomes a value of that Relate type
  (see the table below), and an array of strings with one such tag becomes a list of that type.
- **The top-level record** `{kind: "property-graph", name: "roster"}`: checked, and its properties are
  reported as not stored.
- **Comments**, `//` and `/** ... */`: ignored.

### Typed values

| Tag | Relate type | Write | Example |
|---|---|---|---|
| `date` | DATE | `YYYY-MM-DD` (a year of four or more digits, optionally negative) | ``date`2024-01-15` `` |
| `time` | ZONED TIME | `hh:mm:ss`, a fraction of 1 to 9 digits if you like, then `Z` or `+hh:mm` | ``time`10:30:00+01:00` `` |
| `localtime` | LOCAL TIME | `hh:mm:ss` with the optional fraction, no offset | ``localtime`10:30:00` `` |
| `datetime` | ZONED DATETIME | a date, `T`, a time with its offset, and optionally a zone name in square brackets | ``datetime`2024-01-15T10:30:00+01:00[Europe/Paris]` `` |
| `localdatetime` | LOCAL DATETIME | a date, `T`, a local time | ``localdatetime`2024-01-15T10:30:00` `` |
| `duration` | DURATION | an ISO 8601 duration with an upper-case `P`; a number may have a leading `-`; only seconds may have a fraction | ``duration`P1Y2M3DT4H5M6.789S` `` |
| `point` | POINT | extended well-known text with an SRID: 4326 or 7203 with `POINT(x y)`, 4979 or 9157 with `POINT Z(x y z)`; geographic points are longitude then latitude | ``point`SRID=4326;POINT(-74.0 40.7)` `` |
| `uuid` | UUID | 32 hexadecimal digits in 8-4-4-4-12 groups | ``uuid`123e4567-e89b-12d3-a456-426614174000` `` |

Tags are lower case and matched exactly; any other tag (`md`, `Date`, `vector`) is dropped and reported as
before. A recognized tag with content that does not follow its form is an error, never a plain string.

The forms are ISO 8601 extended format, narrowed (seconds are required; second 60 and `24:00:00` are not
accepted), with three extensions that Relate itself reads and prints: a negative year, a zone name in
brackets after a datetime's offset (RFC 9557 notation), and a sign on each number of a duration, because a
duration keeps months, days and nanoseconds apart and their signs can differ. A leading `-` before the `P` of
a duration, weeks, and a fraction on anything but seconds are not accepted. `check` verifies the form
and the ranges of every typed value without a database; whether a zone name exists, and whether the offset
is that zone's, is only known when the file is loaded.

Vectors cannot be written in gram, in any form: exporting a database with a vector property fails and
names it.

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
| `{h: 168cm}`, `{w: 3.5kg}` | `value.measurement` | Store a number and a unit (`height: 168, unit: "cm"`) |
| `{m: {k: 1}}` | `value.map` | Flatten the entries into separate properties |
| `{o: 010}` | `value.octal` | Write the value in decimal (`8`) or hexadecimal (`0x8`) |
| `{n: 9223372036854775808}` | `value.integer-range` | Integers must fit in 64 bits; use a string for larger numbers |
| `{xs: [1, "a"]}`, `{xs: [1, 2.5]}` | `value.array-mixed` | An array holds one kind of value; write every element as that kind (`[1.0, 2.5]`) |
| ``{xs: [date`2024-01-01`, "x"]}`` | `value.array-mixed` | An array of typed values uses one tag throughout |
| ``{when: date`2024-13-45`}``, ``{t: time`10:30`}`` | `value.tag-content` | Write the content in the form the tag takes (see Typed values above); the message says what is wrong |
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

## Fixing a file automatically

`relate-gram fix` applies the corrections above to the file's text, so you do not have to edit hundreds
of lines by hand. It never changes your file: by default it writes `people.fixed.gram` next to it, and it
will not replace an existing file of that name. Everything outside the corrected text, including
comments and line endings, stays exactly as it was.

```bash
relate-gram fix people.gram --rel-type KNOWS --direction right
# people.gram: applied 5 fix(es) in 2 round(s):
#   2 x rel.no-type: insert the relationship type KNOWS
#   ...
# wrote people.fixed.gram
relate-gram check people.fixed.gram        # compare the two files in your editor or with diff
```

| Error | What `fix` does | Needs |
|---|---|---|
| `rel.no-type` | Inserts the type: `-->` becomes `-[:KNOWS]->` | `--rel-type NAME` |
| `rel.undirected` | Adds an arrowhead: `--` becomes `-->` (right) or `<--` (left) | `--direction right\|left` |
| `rel.bidirectional` | Removes one head: `<-->` becomes `-->` or `<--` | `--direction right\|left` |
| `rel.arrow-family` | Writes a thin arrow: `==>` becomes `-->` | nothing |
| `construct.double-colon` | Writes `:` for `::` | nothing |
| `value.octal` | Writes the value in decimal: `010` becomes `10` or `8` | `--octal decimal\|octal` |
| `value.array-mixed` | Writes the integers of a number array as decimals: `[1, 2.5]` becomes `[1.0, 2.5]` | nothing |
| `value.tag` | Removes the tag (with `--tags error`) | nothing |
| `value.tag-content` | Nothing: the intended value is unknown, so the error is annotated | nothing |

A correction that needs a choice is applied only when you give it, and then to every occurrence in the
file; relationships that need different types need hand edits. When a fix reveals another (a direction,
then a type), `fix` repeats until nothing more applies. Errors it cannot fix, such as syntax errors,
annotations and ranges, are listed against the new file with their new line numbers, and each gets a comment
above its line in the new file:

```gram
// relate-gram: error[value.range]: the range `1..2` has no Cypher type; write the bounds as separate properties
(c {r: 1..2})
```

The comment is harmless (the loader ignores comments), the next run of `fix` replaces it, and
`--no-annotate` leaves it out. If you re-run `fix` with more options after seeing the errors, add `--force`
to replace the earlier output.

Other ways to write the result: `-o FILE` (or `-o -` for standard output), and `--in-place`, which
replaces your file after copying it to `FILE.orig`.

## Why so strict

A loaded database that quietly differs from its file cannot be trusted. Each rule above exists because
the alternative is to invent something the file does not say: a default relationship type, a direction
for an undirected arrow, a number from an octal literal. The reasons are in the ADRs, in
[`design/adr/`](../design/adr/) (ADR-001, 006, 007 and 008).

If a rejection is in your way, the error message is the shortest path to a file that loads. `relate-gram fix`
can apply many of the corrections for you (see above).
