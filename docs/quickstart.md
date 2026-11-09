# Five-minute quickstart

You have a gram file and want its graph in Relate. This uses the `relate-gram` command and Relate's own
`relate` command; you need neither Rust code nor Cypher to load.

## 1. Build the command

```bash
git clone --recurse-submodules https://github.com/relatedb/relate-gram
cd relate-gram
cargo build --release          # the command is target/release/relate-gram
```

## 2. Write a gram file

`people.gram`:

```gram
(ada:Person {name: "Ada", born: 1815})
(grace:Person {name: "Grace", born: 1906})
(alan:Person {name: "Alan"})
(ada)-[k1:KNOWS {since: 1840}]->(grace)
(alan)-[k2:KNOWS]->(ada)
```

Every relationship needs a type and a direction (`-[k1:KNOWS]->`); see
[what is supported](supported-gram.md). The `ada`, `k1` and similar names are identifiers: they let other
lines refer to an element, and they are how a later load recognizes it.

## 3. Check it

```bash
relate-gram check people.gram
# checked 1 file(s): 0 error(s), 0 warning(s)
```

`check` reads the file and reports every problem with the line, a marker under the text, and what to
write instead, without touching a database. Try breaking the file (replace a relationship with `(ada)-->(grace)`) and run it
again:

```
error[rel.no-type]: a relationship needs a type; write one, for example `-[:KNOWS]->`
 --> people.gram:4:6
  |
4 | (ada)-->(grace)
  |      ^^^
```

(Put the line back before you go on.)

## 4. Load it

```bash
relate-gram load people.gram --db ./people
# people.gram: 3 node(s) and 2 relationship(s) created, 0 and 0 updated
```

`./people` is a Relate database directory; it is created if it does not exist. A load either completes
or changes nothing.

## 5. Query it

With the [`relate` command](https://relatedb.ai/):

```bash
relate query --database ./people \
  "MATCH (a)-[r]->(b) RETURN a.name AS from, type(r) AS rel, b.name AS to"
# rows[2]{from,rel,to}:
#   Ada,KNOWS,Grace
#   Alan,KNOWS,Ada
```

## 6. Edit and reload

Change a property in `people.gram` and load it into the same database. Elements with an identifier
(`ada`, `grace`, `alan`, `k1`, `k2`) are matched to the stored ones, so nothing is duplicated and your
edit is applied:

```bash
relate-gram load people.gram --db ./people
# people.gram: 0 node(s) and 0 relationship(s) created, 3 and 2 updated
```

An element written without an identifier cannot be recognized, so it is created again on every load.
`load` says how many such elements it loaded.

## 7. Write it back out

```bash
relate-gram export --db ./people > people-out.gram
relate-gram normalize people.gram        # the same, from a file, without a database
```

Exports are deterministic: the same database always gives the same text.

## Where next

- [What is supported, and why](supported-gram.md): every construct that is rejected, its error code,
  and what to write instead.
- `relate-gram --help` lists the options (identity properties, tag policy, identifier modes).
- The library is `relate_gram::{check, load, export}`; see the [README](../README.md).
