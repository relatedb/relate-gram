# Contributing

relate-gram follows a design process before code: an RFC for the problem, an ADR for each technical
decision, then a specification, plan and tasks (Spec Kit). The rules are in
[`.specify/memory/constitution.md`](.specify/memory/constitution.md), and the documents are in `design/`
and `specs/`. A bug fix or a documentation change does not need an RFC; a new rejection rule, a new
command or a change to what is stored does.

## Working on the code

```bash
git clone --recurse-submodules https://github.com/relatedb/relate-gram
cargo test                          # contract, position, round-trip, corpus, command and fixer tests
cargo fmt --check && cargo clippy --all-targets -- -D warnings
scripts/check-public-api.sh         # only Relate's public interface is used
```

- Write the test first and watch it fail (Constitution VI). A rejection comes with a fixture and its
  expected position (`tests/fixtures/invalid/`), and a new error code with a row in
  `docs/supported-gram.md` and a decision whether it has a fix (ADR-010).
- Cite the ADR a change implements in the commit message.
- `CLAUDE.md` has the project conventions in more detail.

Releases are described in [`RELEASING.md`](RELEASING.md).
