# Data Model: The Conformance Collection

No stored entity. The shapes are those of ADR-015 section 2.2; this file records the runner's view of them.

- **Case**: `{kind, id, requires: [name], settings: {key: scalar}, transaction: bool, expectations: [Expectation], files}` read from `case.gram` and the files beside it.
- **Expectation** (a node of `case.gram`): `Error` and `Warning` with `code` and, when positioned, `line`, `column`, `offset`; `Counts` with the keys of ADR-015 section 2.2; `Applied` with `code`, `count`; `Remaining` with `code`, `line`, `column`, `offset`.
- **Graph line**: the string a node or relationship reduces to (research R4). A graph is its two sorted lists.
- **Probe result**: `{capability: bool}` for the pass, written into the results file.
- **Result**: `(:Result {case, status, version})` with `status` one of `passed`, `failed`, `skipped`, `not-applicable`.
- **Known failure**: `(:KnownFailure {case, reason, issue})` in `conformance/known-failures.gram`.

| Kind | Files | Runs |
|---|---|---|
| `reject` | `case.gram`, `input.txt` or `input.bin`, optional `start.cypher` | `check` and `load` |
| `load` | `case.gram`, `input.txt`, `expected.gram`, optional `start.cypher`, optional `exported.txt` | `load`, and `export` with `exported.txt` |
| `export` | `case.gram`, `start.cypher`, `exported.txt` or `Error` nodes | `export` |
| `fix` | `case.gram`, `input.txt`, `fixed.txt` | `fix` |
