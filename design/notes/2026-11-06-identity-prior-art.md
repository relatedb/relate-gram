# Identity and reload: prior art survey

Date: 2026-11-06. Informs RFC-003 (named graphs and reloading changed files). Non-binding.

## Question

When a user loads a gram file, edits it and loads it again, what identifies an element across loads, what happens to elements that appear, vanish or never had an identifier, and how is deletion limited to what the loader created? This note collects how other systems answer, and what that suggests for relate-gram.

## Method and reliability

Three parallel surveys: declarative apply tools, RDF and graph databases, and the local Relate, pattern-rs and tree-sitter-gram repositories. Web pages were read through a summarizing fetch tool that truncated some of them, so the web findings are second-hand. Items marked **[unverified]** come from background knowledge, not a fetched page. Dgraph and GraphML were not verified. Relate findings come from its own docs and tests; nothing was run.

Each survey stopped early once the same patterns recurred across its first three systems.

## 1. Declarative apply tools

### kubectl apply (and server-side apply)

- **Identity:** kind, namespace and name written in the manifest. Objects with only `generateName` cannot be tracked by apply **[unverified]**.
- **Update:** client-side apply does a three-way merge of the last-applied annotation, the file and the live object. Fields in the annotation but missing from the file are deleted. Fields set only by other writers are kept. Server-side apply records an owner for each field (`managedFields`) and rejects a conflicting apply unless forced.
- **Removal:** neither mode deletes objects dropped from the files by default. `--prune` is opt-in, alpha, and limited by a selector plus the last-applied annotation, or by an applyset label (`applyset.kubernetes.io/part-of`) and a parent object. An object belongs to at most one set.
- **Renames:** a new name is a new object, and the old one stays until pruned.
- **Complaints:** a wrong selector or scope when pruning deletes or keeps the wrong objects; mixing apply with create or replace loses the annotation.

### Terraform

- **Identity:** the resource address (`TYPE.LABEL`, plus the `count` or `for_each` key), bound to one remote object in state.
- **Update:** configuration wins; out-of-band changes are reverted. There is no merge with other writers.
- **Removal:** a block removed from the configuration means destroy. `lifecycle { destroy = false }` drops it from state and leaves the object. Terraform never touches objects with no state entry.
- **Renames:** a rename without a `moved` block is destroy plus create. Adopting an existing object needs an explicit `import` block.
- **Complaints:** an accidental rename destroys data; state can be lost or drift.

### Helm

- **Identity:** release name and namespace, with Kubernetes identity for objects inside.
- **Update:** three-way strategic merge preserves live-only additions.
- **Removal:** objects in the previous release but not the new one are deleted on upgrade **[unverified]**. Ownership is recorded as metadata on each object. By default Helm refuses to touch an existing object it does not own; `--take-ownership` overrides, and `helm.sh/resource-policy: keep` protects an object.
- **Renames:** delete plus create.

Sources: kubernetes.io declarative-config and server-side-apply docs; developer.hashicorp.com Terraform refactoring, `removed`, import and state docs; helm.sh upgrade, advanced-topics and hooks docs.

## 2. RDF, JSON-LD and graph databases

### RDF

- Blank-node labels are "always locally scoped to the file" and "not persistent or portable" (RDF 1.1 Concepts). Reloading a file produces new blank nodes unless they are skolemized.
- Skolemization replaces a blank node with a fresh globally unique IRI. The spec does not say how to generate it.
- Canonical labelling (RDFC-1.0) hashes each blank node's neighbourhood and numbers them `c14n0`, `c14n1`, and so on. It is deterministic, but a change anywhere connected to a node can change its label, and the spec warns that a diff of canonical labels can show more change than happened. Some inputs make the algorithm run very long.
- Named graphs are separated by a graph name (an IRI or blank node), not by element identifiers. Blank nodes can be shared between graphs in a dataset.

### JSON-LD

- `@id` holds an IRI or a blank-node identifier (`_:x`), which "have no intrinsic meaning". A node object without `@id` is an unnamed blank node. Flattening generates `_:b0`, `_:b1` by counter **[unverified]**.
- `@graph` with an `@id` is a named graph.
- Repeat loading is unspecified; blank-node labels are not stable across loads.

### Neo4j

- Identity is whatever the user `MERGE`s on. `elementId()` is only guaranteed unique within a transaction, and internal ids are reused; the manual recommends application-generated ids.
- `MERGE` matches a whole pattern or creates the whole pattern. `MERGE` on a null property is an error. A relationship pattern with unbound nodes creates new nodes if the full pattern is not found.
- A composite node-key constraint requires every property and a unique combination. A composite uniqueness constraint exempts a node that lacks one of the properties. Anonymous nodes therefore pass silently and duplicate on every load.
- APOC GraphML export writes per-file ids (`n188`) that are not stable. Import with `storeNodeIds` keeps them, and the docs strongly recommend unique constraints. Neo4j separates graphs by database, not by named graph.

Sources: w3.org rdf11-concepts, rdf-canon and json-ld11; neo4j.com Cypher manual (MERGE, constraints, scalar functions) and the APOC GraphML page.

### Not verified

Dgraph blank nodes (`_:name`) are scoped to one mutation and the response returns the new uids, with upsert blocks to reuse nodes **[unverified]**. GraphML node ids are unique within a document and meaningless across files **[unverified]**. `neo4j-admin import` ID spaces were not confirmed.

## 3. Relate and the gram repositories

Relate (paths under `neo4j-labs/cypherlite`):

- **Composite constraints:** `UNIQUE` and `NODE KEY` over several properties exist (`docs/reference/cypher-syntax.md`). For `UNIQUE`, a tuple with any null or missing component is never probed and never conflicts (`docs/adr/0172-incremental-constraint-validation.md`, `docs/adr/0439-commit-time-uniqueness.md`). For `NODE KEY`, a missing property is an error. Tests cover the single-property cases (`relate/relate/tests/constraint_validation/matrix.rs`); a dedicated composite null test was not found. The in-repo TCK pin was retired, and no constraint scenarios were confirmed.
- **Relationship constraints:** supported, with no backing index, so checks scan (`docs/adr/0172`).
- **`STARTS WITH`:** `docs/adr/0030` maintains a sorted prefix map per label and key and seeds the predicate from it. Whether explicit `CREATE INDEX` indexes serve it is not confirmed.
- **Property keys:** backtick quoting is documented. Nothing confirms `@`-prefixed keys, and no key-character restriction is documented.
- **Ids:** `id()` is an "Internal ID" from persisted counters; no ordering or no-reuse guarantee is documented.

pattern-rs and tree-sitter-gram:

- `ReconciliationPolicy` (`pattern-core/src/reconcile.rs`) offers `LastWriteWins`, `FirstWriteWins`, `Merge(...)` and `Strict`. `StandardGraph` uses `Merge(UnionElements, ...)` and creates placeholder nodes for relationship endpoints seen before their declaration. Anonymous elements carry an empty identity (`Symbol("")`); how reconciliation treats them is not confirmed.
- Neither repository defines named graphs, graph names, or identifier scope across files. The leading record is a document header; `kind` is a convention. `graph-name.kind.gram` and `property-graph` are our own conventions, not upstream ones.

## 4. pattern-hs: survey and RFC-012

The Haskell reference implementation of pattern (`relateby/pattern-hs`) holds a survey of identity management (`proposals/research/identity-management-survey.md`, 2026-07-03) and a draft RFC for scoped identity (`proposals/rfc/RFC-012-scoped-identity.md`, 2026-07-06). Both were read directly from the local files. The survey says its RDF and Datomic claims were checked against specifications and that the rest were fetched from primary sources but not formally verified.

### What the survey adds

- **Datomic tempids.** Local ids resolve to permanent ids at the transaction boundary, and the transaction report returns the mapping from each local id to what it became. A reload summary can do the same.
- **Upsert on a unique key.** `:db.unique/identity` resolves a record carrying an existing key to the existing entity. It is the one mechanism in the survey that gives both idempotent reload and deliberate sameness across files.
- **Mutable things need a stable anchor, not a hash.** Perkeep's permanode is the example; content addressing suits immutable values.
- **Deterministic generated ids.** UUIDv5 hashes a namespace and a name. It is only as stable as the name, so ordinal names shift when the file is edited.
- **Scope as authority, not location.** Matrix ids and XML namespaces show the form `scope:local`, where the scope names whoever allocated the local part.
- **Late sameness.** Wikidata merges two items and leaves a redirect; it never deletes or reuses the losing id.
- **Neo4j import id-groups** give per-source id spaces, and a composite node key has the `(scope, local)` shape.

### What RFC-012 proposes

- Identity is the pair `(scope, local)`, grounded at a frame, the scope-bearing container.
- A file declares its scope in the leading record under the reserved key `namespace`. The value `@org/sub` is authority-qualified and safe to share across stores; a bare value is local to one store. The reserved-key registry already holds pattern-rs's `kind` and `variant`.
- With no `namespace`, the store assigns a scope at ingest. That isolates the file's ids but gives no idempotency, since a repeat ingest mints a fresh scope.
- Anonymous elements are weak entities: identified by their nearest named ancestor plus an ordinal. They never merge by identity, and they cannot be the target of a reference until promoted.
- Reconciliation has to treat anonymous elements as distinct. The parser currently represents them as the empty symbol, so keying on identity would put all of them in one bucket.
- Merging sameness discovered later belongs to reconciliation, with a redirect.
- A notation for referring from one file into another's namespace is deferred.

### Where it meets relate-gram

- **Header key.** ADR-003 reads the graph name from `name`; RFC-012 uses `namespace`. A file declaring only `namespace` would be named from its file name here, and a user's own `name` property (for example a title) would be read as the graph name.
- **Names that start with `@`.** ADR-003 §2.3 says `@@` falls under the reserved `@` prefix, so no user graph can have that name. RFC-012's authority-qualified scopes start with `@`.
- **File name as fallback.** RFC-012 rejects a file path as scope because moving the file breaks identity. ADR-003 uses the file name when neither the caller nor the record gives a name.
- **Shape of identity.** The pair `(scope, local)` supports two properties (`@graph`, `@id`) over one combined value, since an authority-qualified scope contains `@` and `/` and a plain gram symbol allows only one of them.
- **Anonymous relationships.** relate-gram graphs are flat, but an anonymous relationship has named endpoints that could play the role of the owner: identity from (start, type, end) plus an ordinal among identical parallel relationships. This is a candidate of our own, not in RFC-012.
- **Empty-symbol hazard.** pattern-rs's `gram-codec` also represents anonymous elements as `Symbol("")`. Whether its reconciliation merges them is not confirmed; it matters if the loader reuses that code.

## 5. Patterns across systems

1. **File-local labels are never persistent identity** (RDF blank nodes, JSON-LD blank nodes, GraphML ids, Dgraph blank nodes). Persistence needs an explicit step: skolemization, storing the ids, or a user-chosen key.
2. **Stable identity comes from an explicit key** written in the file or chosen by the user (kind/namespace/name, resource address, IRI, `@id`, a `MERGE` key). Position and content do not identify anything.
3. **A rename is delete plus create** unless the user states an alias (Terraform `moved`). Nothing detects renames automatically.
4. **Deletion is scoped by provenance the tool records** (last-applied annotation, applyset label, Terraform state, Helm ownership metadata) and is opt-in or guarded. Objects the tool does not own are not deleted.
5. **Adopting existing objects is explicit** (Terraform `import`, Helm `--take-ownership`).
6. **Updates merge per field and keep other writers' fields.** kubectl removes only fields it previously set, which it can do because it stores a copy of what it last applied. Terraform is the exception, since configuration always wins.
7. **Anonymous elements are the weak point.** Kubernetes cannot track `generateName` objects through apply. Generated labels for anonymous elements are counters (`c14n0`, `n188`, `_:b0`), and they are stable only for one load or one exact input; hash-derived labels shift when anything connected changes. Neo4j's composite uniqueness constraint ignores elements with a missing key property, so anonymous nodes duplicate on each load.
8. **Several graphs are separated by a graph name, not by element ids** (RDF datasets, JSON-LD `@graph`) or by separate databases (Neo4j).
9. **The author declares the scope, with a fallback that isolates but does not give idempotency** (pattern-hs RFC-012, RDF named graphs, Datomic upsert on a declared key). A scope derived from location is treated as fragile.
10. **Anonymous elements are identified by their owner plus an ordinal** (pattern-hs weak entities; RDF blank nodes nested under a named node), and the survey states no clean answer to ordinal shifts when the file is edited.

## 6. What this suggests for RFC-003

| Question | Finding |
|---|---|
| Match on the file's own identifiers (5.2) | Pattern 2 in every system. |
| Additive reload, prune opt-in and provenance-scoped, preview (5.3) | Patterns 4 and 5; kubectl, Terraform and Helm all guard deletion and limit it to what they own. |
| Never adopt elements the loader did not create | Pattern 5. Elements without membership are never touched. |
| Rename is remove plus add, with an explicit alias later (7.7) | Pattern 3. |
| Removed property in the file (7.3) | kubectl removes dropped fields only because it stores the last-applied copy. Without a stored copy, relate-gram cannot tell which properties the file set. Keeping them is the default; removal needs extra provenance. |
| Anonymous elements: match on live content | Pattern 7 argues against generated counters and hashes as matching keys. |
| Two properties (`@graph`, `@id`) against a combined `@id` | Pattern 8 favours a separate graph name over a prefix inside the identifier. Relate's composite `UNIQUE` ignores missing components, which fits two properties with anonymous elements leaving `@id` unset. Both facts are documented, not tested. |
| Header key for the graph name (`name` or `namespace`) | Pattern 9 and RFC-012's reserved-key registry. Open; see RFC-003 7.12. |
| Anonymous relationships identified by (start, type, end, ordinal) | Pattern 10 applied to a flat graph. A candidate of ours, not tested against any system. |
| Persistent counter for generated identifiers | No surveyed system stores such a counter; the surveyed counters are per-load. It remains a design of ours, for export stability. |

## 7. Candidates for a spike

Questions the repositories could not settle, to be tested against a live Relate:

- Composite `UNIQUE` and `NODE KEY` with one property missing: ignored or error.
- Composite constraints on relationships.
- `@graph` and `@id` as property keys: create with backticks, match, set, index and constrain, including a composite (`@graph`, `@id`).
- Whether `STARTS WITH` uses an index, with and without `CREATE INDEX` (`EXPLAIN`).
- `id()` and `elementId()` across deletion and reload, and whether ids are reused.
- How pattern-rs `StandardGraph` and `reconcile` treat empty-identity (anonymous) subjects. pattern-hs RFC-012 reports that identity-keyed reconciliation there merges them into one bucket, and `gram-codec` uses the same empty symbol.
