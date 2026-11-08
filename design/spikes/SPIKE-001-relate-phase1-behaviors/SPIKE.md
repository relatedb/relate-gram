---
id: SPIKE-001
title: Relate Behaviors Assumed by Phase 1
status: done
rfcs: [RFC-003, RFC-001]
created: 2026-11-08
---

# SPIKE-001: Relate Behaviors Assumed by Phase 1

## 1. Question

ADR-001 and ADR-002 rely on five behaviors of Relate (the embedded Cypher database, crate `relate`) that nobody has run. Which hold?

1. **`@id` as a property key.** Can a property be named `@id` (backtick-quoted in Cypher, or set from a parameter map): created, matched, returned, listed in `properties(n)`, indexed, constrained?
2. **`id()` order.** For one sequence of creations, is `id(n)` / `id(r)` the same in two fresh databases, and does it ascend in creation order? Are ids reused after deletion? (ADR-002 §2.4 orders exports by it.)
3. **Integer versus string identity.** ADR-001 treats the integer `42` and the string `"42"` as one identifier by text. How do `MATCH`, `MERGE` and `toString()` behave on `@id` and on a user identity property holding either?
4. **Atomic load.** Through the public library (`Connection::begin`, `commit`, `rollback`), does a multi-statement load roll back completely after an error part-way, and does a failed statement leave the transaction usable?
5. **Identity constraints and indexes.** For the phase 1 identity property: does composite `UNIQUE` ignore missing components, does `NODE KEY` error on missing ones, do relationship constraints work, and does a constraint or index back lookups (plan shows an index seek)?

Resolves the assumptions in ADR-001 §2.7 and §2.10 and ADR-002 §2.4, and RFC-003 questions about index-backed identity.

## 2. Method

A throwaway Rust binary, `scripts/probe`, depends on the public `relate` crate by path (`/Users/abk/Developer/neo4j-labs/cypherlite/relate/relate`, commit `84859b14`) and runs each probe against a fresh `Database::open(":memory:")`, printing every statement and its result or error. Only public API is used (Constitution V). Run with `cargo run --release --manifest-path design/spikes/SPIKE-001-relate-phase1-behaviors/scripts/probe/Cargo.toml`. The full output of the run reported below is `probe-output.txt`. Probes 6 and 7 were added after the first run, to find out what backs `MERGE` and to time the match strategies at 50,000 nodes.

## 3. Time-box

One day. Anything that needs private engine access is recorded as "not answerable through the public interface" and not pursued.

## 4. Findings

Relate 0.16.1, in-memory database, one run (`probe-output.txt`).

### 4.1 `@id` is an ordinary property key — holds

- With backticks, `@id` can be created, matched (`WHERE` and inline map), returned, listed in `keys()` and `properties()`, set from a parameter map (`SET n = $props`), put on relationships, and used in `CREATE INDEX` and `CREATE CONSTRAINT ... IS UNIQUE` for nodes and relationships. `@hash` and `@@` work as keys too.
- A uniqueness constraint creates its own index (`SHOW INDEXES` lists it with the constraint as owner), so `CREATE INDEX` followed by `CREATE CONSTRAINT` on the same property fails with `AlreadyExists`. A loader that wants a constraint must not also create an index.

### 4.2 `id()` is deterministic and ascends — holds, with a caveat

- Two fresh databases given the same creation sequence (single `CREATE`s, one `UNWIND ... CREATE`, relationships) produced identical ids; node ids ascended in creation order, and relationship ids likewise (separate counters, both from 0).
- After deleting two nodes and creating two more, the new ids were 10 and 11: the deleted ids 2 and 3 were not reused. An unordered scan returned the same order as `ORDER BY id(n)`.
- Not tested: ids after a checkpoint and reopen, or across concurrent writers. ADR-002 §2.4 already says the order carries no meaning beyond reproducibility.

### 4.3 Integer and string are different values — does not hold as ADR-001 assumes

- `n.`@id` = 42` does not match a stored `'42'`, and `MERGE (n:P {`@id`: 42})` created a second node next to `'42'`.
- A `UNIQUE` constraint treats `1` and `'1'` as distinct values (both were accepted).
- `toString(n.uid) = '42'` finds both an integer and a string `42`, but the plan is a label scan, not an index seek. `n.uid IN [42, '42']` and `n.uid = 42 OR n.uid = '42'` are also label scans.
- So "equal by text" is a rule the loader has to implement itself. `@id` is always a string (ADR-001), so there it is automatic. For a configured identity property, the database enforces neither text-uniqueness nor an index-backed text lookup.

### 4.4 Transactions: atomic, but a failed statement poisons the transaction

- `begin`, then statements, then `rollback` discarded everything, including a schema statement run inside the transaction. Dropping a connection with an open transaction rolled it back. Uncommitted writes were invisible to another connection; after `commit` (bookmark `relate:1`) they were visible.
- After a failed statement (constraint violation, or an invalid property such as a nested list, `InvalidPropertyType`), `in_transaction()` stays `true`, but every later statement fails with `transaction failed; rollback required`. Only `rollback` is useful. A load that errors part-way therefore needs no cleanup code beyond calling `rollback`.

### 4.5 Constraints — holds as ADR-001 §2.10 assumes

- Single-property `UNIQUE` and composite `UNIQUE` ignore elements where a component is missing: two `(:A)` and two `(:B {k1: 1})` were accepted.
- `NODE KEY` rejects an element with a missing component (`requires properties`) and a duplicate.
- Relationship `UNIQUE` and `IS NOT NULL` constraints work. Creating a constraint over existing duplicate data fails.
- A constraint is per label: it did not see `(:M {id: 20})` next to `(:A:M {id: 20})`.
- Lookups by label and property, including composite, use `NodeIndexSeek`. A relationship lookup by property plans as `Expand` (no relationship index seek).

### 4.6 Matching by `@id` without a label — a gap in ADR-001

ADR-001 §2.7 matches an existing element by `@id` and unions labels, so a file may add a label to an element. An index seek needs a label, so the match cannot use one:

- `MATCH (n) WHERE n.`@id` = 'x'` plans as `AllNodesScan`. 2,000 such lookups against 50,000 nodes took 53.9 s (quadratic).
- `MERGE (n:A {`@id`: 'z'})` does not find an existing node that has `@id` 'z' but no label `A`; it created a second one.
- What is fast: one scan returning every `id(n)` and `@id` (50,000 rows in 66 ms); 50,000 updates addressed by `WHERE id(n) = i` from an `UNWIND` (207 ms); 50,000 `MERGE (n:A {`@id`: ...})` with a unique constraint on `:A(@id)` (372 ms).

## 5. Conclusion

- **ADR-001 §2.7 needs a matching strategy.** Matching by `@id` must not rely on labels, and a label-free lookup is a full scan. Reading every existing `@id` once at the start of a load (about 66 ms per 50,000 nodes), holding the map in the library, and then writing by `id(n)` in batches is fast and keeps label union. The same read also detects duplicate `@id` values (ADR-001 §2.7's error). Per-element `MERGE` is only usable if matching is restricted to one label, which would drop label union. Suggested text for ADR-001 §2.7: "A load reads the existing `@id` values once, before writing, and addresses matched elements by their internal id."
- **ADR-001 §2.10 (identity property)** has the same constraint, plus a type issue: matching by text needs the loader to treat `42` and `"42"` as one identity, which the database does not. Either the loader scans and compares text itself (as above), or the ADR requires identity-property values to be strings (an error on integers), which loses nothing a user can express except bare integer keys. Decide in the ADR.
- **ADR-001 §2.7 atomicity** is satisfiable with the public interface: one `begin`, all statements, `commit`, and `rollback` on any error. Add that a failed statement leaves the transaction unusable except for rollback.
- **ADR-002 §2.4** stands: `ORDER BY id(n)` is stable and reproducible; the deleted-id observation can be added to its consequences.
- **No uniqueness constraint is needed for phase 1.** The map-based load does not depend on one, and a constraint would also block loading a graph that legitimately holds duplicate identities that ADR-001 reports as an error with a better message. A constraint on `@id` is an optional later optimization for label-scoped lookups.
- Open: scale beyond 50,000 nodes (memory for the id map), and behavior after reopening a persisted store, were not tested.
