---
id: RFC-003
title: Named Graphs and Reloading Changed Files
status: draft
created: 2026-11-06
adrs: []
---

# RFC-003: Named Graphs and Reloading Changed Files

## 1. Summary

A gram file is something people edit. They load it into a database, change a property, add a node, and load it again, expecting the database to now match the file. A loader that treats every load as new data gives them duplicates, and one that refuses the second load gives them no way forward. Either way the file and the database drift apart.

The mechanism that makes a reload meaningful is identity that outlives a single load. A *named graph* is a set of elements in one Relate database that belong together, usually because they came from one gram file, and that the database keeps apart from other graphs. Elements of a named graph keep the identifier the file gave them, so a later load can tell which elements it already holds. The organizing claim is that the file's own identifiers are the best key for matching: they are explicit, the user already maintains them, and they need no guessing. A reload then becomes an upsert, where elements the file names are created or updated, and nothing else is touched unless the user asks.

This RFC sets the direction for named graphs: how elements record which graph they belong to, what a reload does by default, which options change it, and what that means for export. It also identifies a conflict with the current rule that identifiers are dropped after a load, which the follow-up decisions must resolve.

## 2. Problem / Motivation

**Reloading an edited file has no good answer today.** Take a file `people.gram`:

```
(ada:Person {name: "Ada", age: 30})
(grace:Person {name: "Grace"})
(ada)-[:KNOWS]->(grace)
```

The user loads it, changes `age: 30` to `age: 31`, adds `(linus:Person {name: "Linus"})`, and loads it again. What the loader can do now:

- Load without remembering anything. Gram identifiers are dropped after a load, so nothing says that `ada` was already loaded. The second load creates a second Ada, a second Grace and a second relationship. The database holds both versions side by side.
- Compare a digest of the file. The meta node can record a hash of the loaded bytes, so an unchanged file is recognized and skipped. A changed file is a different hash, and the only safe response without element identity is an error.

Neither lets the user do the one thing they wanted, which is to update the database from the edited file.

**Matching without identity is a guess, and wrong guesses are quiet.** Without a stored identifier a loader can match elements on all their properties, which fails for exactly the property the user edited, so every edit makes a new element. It can match on property names that tend to be unique, such as `name`, `title` or anything ending in `id`, which merges two different people called Ada without saying so. Both fail without telling the user.

**A database holds one undifferentiated graph.** Two revisions of one dataset loaded into one database produce a single graph with every person appearing twice. Nothing records which file an element came from, so nothing can export one graph on its own, remove it, or reload it.

## 3. Goals

3.1. Give every element loaded from a named file a record of which named graph it belongs to and which identifier the file gave it.

3.2. Make loading a changed file into a named graph update the graph to match the file, without duplicating elements the file already created and without deleting data the file did not mention.

3.3. Report every reload as a summary of what changed: elements added, updated, unchanged, and present in the graph but absent from the file.

3.4. Let the user choose stricter or more destructive behavior explicitly: previewing a reload without applying it, removing elements the file no longer contains, and replacing a graph wholesale.

3.5. Give files that carry no identifiers, and graphs that need another key, a way to be matched without guessing.

3.6. Give a load that names no graph a default graph, so that every load keeps element identifiers and a reload behaves the same whether or not a name was given.

3.7. Round-trip a named graph through gram, so that an export of a named graph reloads into the same graph without changes.

3.8. State exactly which existing rules this changes, and amend them.

## 4. Non-Goals

4.1. Guessing a match key from property names such as `name`, `title` or `*id`, or from the data's shape.

4.2. Three-way merging against a stored copy of the previously loaded file, which would reconcile file edits with edits made through Cypher. It is more machinery than the problem needs.

4.3. Merging graphs loaded from different files into one graph by similarity.

4.4. Loading into a persistent or non-empty database beyond what named graphs themselves imply. That question belongs to RFC-001 (7.9).

4.5. Cypher-level support for named graphs, such as new query syntax or a graph-scoped query mode. Queries remain Cypher over ordinary properties.

4.6. Language bindings and client structures that expose identity. They build on this and come later.

## 5. Proposed Approach (high-level)

Prior art for these directions is collected in `design/notes/2026-11-06-identity-prior-art.md`, which includes the scoped-identity work in pattern-hs (the Haskell reference implementation of pattern).

### 5.1 Identity and membership on elements

A load that has a graph name keeps what an unnamed load drops. Each element it creates carries two reserved properties: its gram identifier (the `@id` value) and the name of the graph it belongs to. A candidate for the second is a `@graph` property, so that membership is a plain, queryable property like the rest, with no new element types or relationships.

Identity is the pair of graph and identifier. The same identifier in two graphs is two elements, which is what lets one database hold several revisions of a dataset. The pair is kept as two components, not one concatenated string: a graph name can be authority-qualified (for example `@org/census`), containing characters that a plain gram identifier cannot, and the pair has the shape of a composite uniqueness constraint. A single value with the graph name as a prefix remains a candidate (7.1), with anonymous elements marked by an empty identifier. The meta node, which already represents the graph itself, becomes a member of the reserved graph `@@` by the same mechanism, replacing the interim rule of recognizing it by its label.

Loads that give no graph name behave as before. Nothing is stored and a second such load duplicates, as the first load rule intends.

### 5.2 Reload as upsert

Within one file, each element has a single defining site, where its labels and properties are written, and any number of references to it (ADR-001). A file therefore never contradicts itself about an element, and the load has no conflict option.

When the file's graph already exists, the load matches elements by identity and applies the file:

- An element whose identifier is already in the graph is the same element. Its labels are unioned with the file's, and each property its definition states takes the file's value. A reference states nothing and changes nothing.
- An element whose identifier is new is created.
- An unchanged file is recognized by its digest and does nothing.

Across loads the file wins, since a later file that differs from an earlier load is the update the user meant. This replaces RFC-001's statement that a merge into an existing graph uses the in-file defaults, which then made a changed value a conflict error. Properties the file does not mention are left alone, so values added through Cypher survive a reload.

Elements the file writes without an identifier cannot be matched by identity. Two candidate rules apply, by kind of element:

- A **relationship** between identified nodes can be identified by its start, type and end, plus an ordinal among identical parallel relationships in the file. Editing its properties leaves its identity unchanged; only reordering identical parallel copies shifts it. This follows the weak-entity idea in pattern-hs, in which an anonymous element is identified by its owner and an ordinal, with the endpoints in the owner's role.
- A **node**, and a relationship whose endpoints are themselves anonymous, matches on its full content: labels, properties, and for relationships their endpoints and type. An edit to such an element appears as one removed and one added.

The reload reports these cases and suggests giving the element an identifier. Generating identifiers for anonymous elements instead is weighed in 6.12.

### 5.3 Operations

The effect of a load is chosen by the operation the user runs, so it is stated where it is typed. The spellings are settled with the CLI integration (RFC-001 question 7.11); the direction is:

- **`merge`** upserts. Elements the file defines are created or updated, and the graph is created if it does not exist. It is additive: an element that is in the graph but no longer in the file stays, and the summary counts it. The summary also lists, for every element the file wrote, what it matched or created, so the user can see how each local identifier resolved. Cypher's `MERGE` matches on the content of a pattern; this operation matches on the stored identifier.
- **`load`** creates a graph. If a graph of that name already exists, it stops with an error that says to use `merge` to update it or `replace` to overwrite it. A file whose digest matches the stored one is reported as already loaded. In a fresh database `load` and `merge` give the same result.
- **`replace`** makes the graph exactly the file, for test fixtures and similar uses where the file is the whole truth.

Two options apply to these operations. A preview option applies nothing and prints the summary. A prune option on `merge` removes elements that the graph holds as members and that the file no longer names. Because membership records provenance, pruning cannot touch data created in the database by other means.

### 5.4 Other match keys

A graph whose file carries no identifiers needs another key. Two sources are candidates, both explicit:

- A hint given on load, naming a property to match on for a label.
- A uniqueness or node-key constraint declared in the database, which already states that a property identifies an element.

Neither replaces the file's identifiers where they exist, and an element that matches on more than one element under a hint is an error, not a pick.

### 5.5 Export

Exporting a named graph writes each element's `@id` as its gram identifier and its membership decides which elements belong in the file. Since the identifier is recorded, an exported graph reloads into the same graph and updates nothing. Export of several graphs from one database, and what the meta node contributes to each file, follow from the membership property.

### 5.6 Consequences for rules already made

Two existing rules assume identifiers are dropped. The project constitution says identifiers "are not stored", and ADR-001 rejects every property key starting with `@` in a file. The direction here replaces it: every load stores the identifier, because a load that names no graph goes into the default graph (5.7). The keys are still written only by the loader and never read from file content. The constitution has to be amended through an ADR. ADR-001 has been amended so that an element is defined at one site, which removes its conflict option, and its statement that a load never matches existing elements has to be updated again when this direction is decided.

### 5.7 Naming a graph

Identity within a graph only helps a reload if the same graph name arrives each time. The scope is best declared by the author in the file, so that it does not depend on where the file is stored: a name taken from a file path is lost when the file is moved or renamed, and the reload silently creates a new graph. The name resolution already in place (the caller's name, then the record's, then the file name) keeps the file name as a last resort, and the summary says which source was used.

A load that resolves no name, such as input from standard input with no record, goes into a default graph with the reserved name `@default`, so it keeps identity like any other. Repeated nameless loads therefore merge by identifier into that graph, as loading two files with overlapping keys into one table would, and the summary reports every element updated. `load` refuses to create a graph that already exists, which also guards against file names that collide (7.13).

pattern-hs proposes declaring a graph's scope with the reserved record key `namespace`, in a registry shared with `kind` and `variant`, and writing authority-qualified scopes with a leading `@` (`@org/sub`). The key, the handling of names that begin with `@`, and the fate of the file-name fallback are open (7.12 to 7.14).

### 5.8 Phasing

Until identity is stored, a changed file for an existing graph is an error that tells the user to load under another name (ADR-003). The first step of this work is element identity and membership, with reload as an upsert and the summary. Prune, replace, preview and alternative keys follow.

## 6. Alternatives Considered

6.1. **Fail on a changed file.** Rejected as the end state: it is safe but gives the user no way to apply an edit. It is the interim behavior.

6.2. **Load as new elements every time.** Rejected: it duplicates every unchanged element, and the database stops matching its source.

6.3. **Match on all properties.** Rejected: the edited property no longer matches, so each edit creates a new element, and elements that differ only in their changed values cannot be paired.

6.4. **Match on well-known property names.** Rejected: a wrong guess merges two distinct elements without notice, and the loader's strictness elsewhere is about never producing a database that differs from its source silently.

6.5. **Infer keys from indexes.** Rejected: an index speeds lookup and says nothing about uniqueness. A declared uniqueness constraint does state it, which is why constraints are a candidate key source (5.4).

6.6. **Delete what the file no longer contains by default.** Rejected: it would destroy data a user added after loading. Removal is opt-in (5.3).

6.7. **Overwrite the whole graph on every reload.** Rejected as the default: it discards edits made through Cypher. It remains available as an explicit option for fixtures.

6.8. **Three-way merge against a stored previous version.** Rejected for now (4.2): storing and diffing a prior copy of every graph is more than the common case needs.

6.9. **One database per graph.** Rejected: it avoids the identity question by giving up the goal of holding several graphs in one database, where queries can span them.

6.10. **Store the identifier as an ordinary property such as `id`.** Rejected: it collides with user data and cannot be told apart from it. Reserved `@` keys are loader-written and cannot be forged by a file.

6.11. **Use the file's path or name as the only scope.** Rejected as a sole rule: moving or renaming the file creates a new graph without warning. A name declared in the file takes precedence, and whether the file-name fallback stays is open (7.13).

6.12. **Generate identifiers for anonymous elements on load.** A sequential count restarted for each load is rejected: inserting an element shifts every later one, and the reload overwrites the wrong elements. A hash of the element's content is rejected as a matching key: it changes with any edit, identical elements collide, and a stored hash goes stale after a Cypher edit. A count kept in the graph and never reused avoids shifting, but the file does not state those identifiers, so a reload still cannot connect an anonymous element in the file to an existing one. It does keep exported identifiers stable. Whether it earns its place for that alone is open (7.15).

6.13. **Merge definitions across sites and across loads, with conflict errors (the earlier form of ADR-001 and RFC-001 §5.5).** Rejected: the most common use of a reload, changing a value and loading again, would fail by default, and an element's content could be spread across several places in the file.

6.14. **One verb whose behavior depends on flags.** Rejected: the effect hides behind defaults the user has to look up. Separate verbs state it, and a `load` that errors on an existing graph prevents silent collisions.

6.15. **Require a name for every load, or derive one from the content digest, when none is given.** Rejected: requiring a name turns the quickest use, loading from standard input, into an error. A digest-derived name never merges unrelated loads but is unfriendly and leaves an edited nameless file with no way to be reloaded as an update. A default graph follows the same one-graph-per-name rule as every other load, and the summary makes any merge visible.

## 7. Open Questions

7.1. Is membership a property on each element, as proposed, or something else, such as a label, a relationship to the meta node, or a combined `@id` value that embeds the graph name, with an empty identifier for anonymous elements? A separate property keeps a missing identifier distinct from membership and fits a composite constraint, which Relate documents as ignoring tuples with a missing component (not yet tested). A combined value needs one property but gives up a single-property uniqueness constraint and makes `@id` mean two things. The choice affects query ergonomics, index use and export.

7.2. What is the exact format of an `@id` value, and how does it appear in gram, given that a plain symbol cannot start with `@` and that gram identifiers can be integers or quoted names? (This is RFC-001 question 7.2.)

7.3. Does a reload that updates a property take the file's value only for properties the file states, as proposed, or should a removed property in the file also be removed from the graph? What does replace do about properties added through Cypher?

7.4. How does a relationship with an identifier match on reload when its endpoints or type changed in the file? Is that an error, or a remove and an add?

7.5. What constitutes "the same anonymous element" for matching on full content, and how are exact duplicates within one file and one graph counted? Is a relationship between identified nodes identified by (start, type, end, ordinal), as proposed in 5.2, and what does a changed endpoint or type then mean?

7.6. Which constraints count as declared keys: uniqueness constraints, node keys, or both, and what happens to a label with several?

7.7. How are renamed identifiers handled? Is a rename always a remove and an add, or can the user declare one?

7.8. How does a user name the graph when loading, and what does the command line look like for `load`, `merge`, `replace`, preview and prune? (Specified with the CLI integration, RFC-001 question 7.11.)

7.9. Can a loaded graph be listed, exported alone, or removed as a unit, and what interface offers that when Cypher is the only enumeration Relate's public library has?

7.10. What happens when two graphs are exported into one file, and when an element in one graph refers to an element of another?

7.11. What does the meta node's digest do once reload exists? It still shortcuts an unchanged file, but should it also record revision history or the time of the last load?

7.12. Which record key declares the graph's name: `name`, as ADR-003 reads it today, or `namespace`, as pattern-hs proposes? A user's own `name` property (a title, say) would be read as the graph name, and a file that declares only `namespace` would be named from its file name. Aligning needs an amendment to ADR-003.

7.13. Does the file-name fallback stay as a last resort, given that moving or renaming a file silently creates a new graph?

7.14. Can a graph name start with `@`, as authority-qualified scopes do (`@org/sub`), given that the default graph is named `@default`? ADR-003 §2.3 reserves `@@` by saying the `@` prefix is reserved, which assumes no user graph name begins with it; the reservation would be narrowed to `@@`.

7.15. Should a graph keep a count of generated identifiers that is never reused, so that exports of a named graph keep stable identifiers across edits and deletions, and where is that count stored? How does a reload of an exported file whose identifiers were generated behave?

7.16. If the loader reuses pattern-rs's reconciliation, does it merge anonymous elements that carry the empty symbol as identity? pattern-hs reports that its own reconciliation does and requires it to be fixed. This belongs to the parser decision.

7.17. Do files produced by other gram tools commonly split an element's labels and properties across several sites? If so, the single defining site rule (ADR-001) may need a relaxation, such as accepting identical repeats.

## 8. ADRs

_(populated automatically as `/adr-create` links ADRs to this RFC)_
