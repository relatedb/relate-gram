# Launch week, and moving to the public Relate release

Relate launches on Wednesday, and `relate-gram` phase 1 is the first gram support to ship with it. This
note records what the week is for, and what to do after Relate's public release, when `relate-gram`
stops depending on a pinned git commit. It binds no decision; if a step needs one, write an ADR.

## Launch week

Status on 2026-11-09: all five items are done (CI, the `relate-gram` command, rendered diagnostics,
`docs/supported-gram.md`, `docs/quickstart.md`). CI passes on a clean checkout (formatting, clippy and the
whole test suite). While `relatedb/relate` is internal, the build clones it with the organization secret
`RELATE_READ_TOKEN` (read access to that repository, shared with this one); the workflow step that uses it is
skipped when the secret is absent, and nothing is needed once Relate is public, at which point the secret
can be removed. The `perf` job runs only when triggered by hand.

Phase 1 (specification `specs/001-phase1-load-export/`) is the product; the week goes into making it
usable by someone who does not write Rust. In order:

1. **CI green** on a clean checkout: the submodule, the git dependency on Relate, and the pinned
   toolchain. CI has not yet run on the pushed commits.
2. **A small command-line binary** in this crate (`relate-gram check|load|export file.gram`), with
   `check` (parse and plan, no database) first. This is separate from the `relate` CLI integration,
   which is still undecided.
3. **Rendered diagnostics**: file, line, the source line and a caret, and the suggested rewrite. The
   library's spans already carry what this needs.
4. **A "what is supported and why" page**: each rejected construct, its error code, and the one-line
   rewrite. It is the hand-written first version of the fixer (`2026-11-09-fixing-rejected-files.md`).
5. **A five-minute quickstart** that starts with a gram file and a command and ends in a Cypher query.

Held back on purpose: the fixer, named graphs, typed tags, language bindings, WebAssembly, and the
`relate` CLI integration.

Known limits to state in the README at launch: reopening a persisted store, concurrent writers and
WebAssembly are untested; loading is tested to 50,000 elements.

## After Relate's public release: leaving the pinned commit

Today `Cargo.toml` depends on `relate` by git from `relatedb/relate`, pinned to commit `d8b2b620` (Relate 0.1.0 as that repository numbers it; the design was first measured at `84859b14` of the former `neo4j-labs/cypherlite`, 0.16.1, which does not exist in the new repository), because the
`relate` on crates.io is a different, older crate (research R1), whose owner is donating the name to the
project, so the released Relate will take it over and the dependency name stays `relate`. Everything in the design was measured
against that commit (SPIKE-001, SPIKE-002). When Relate publishes its release, move to it:

1. **Confirm the release is the same API.** The library uses only `Database`, `Connection` (`query`,
   `query_with_params`, `begin`, `commit`, `rollback`, `in_transaction`), `Value`, and the `rows` of a
   query result. Check each against the released crate's documentation. `scripts/check-public-api.sh`
   already fails if `src/` reaches for anything else.
2. **Swap the dependency.** In `Cargo.toml`, replace the git entry with
   `relate = { version = "x.y", default-features = false }`. Check the feature names again: the pinned
   commit has a default `parquet` feature that this crate switches off. Update `Cargo.lock`.
3. **Check the toolchain.** Read the release's `rust-version`; `rust-toolchain.toml` pins 1.96.0 and the
   crate says 1.90, and the change may let either move (research R2).
4. **Re-run the evidence, not just the tests.** `cargo test` covers the behavior the loader relies on
   (deterministic `id()` order, a failed statement poisoning the transaction, `@id` as a key, integer
   versus string identity). Also re-run the two spike probes against the release, because they test the
   same assumptions directly and also cover value-type edges (silent integer and float list
   coercion, null not stored): `design/spikes/SPIKE-001-relate-phase1-behaviors/scripts/probe` and
   `SPIKE-002-relate-property-values/scripts/probe`. Compare their output with the recorded
   `probe-output.txt`. Re-run the performance test (`tests/perf.rs`, `cargo bench`).
5. **Decide the version policy.** Relate is pre-1.0, so use a tilde requirement on the minor version
   (`~x.y`) so a Relate release cannot change this crate's behavior without a deliberate bump, and say in
   the README which Relate versions are tested.
6. **Make the crate publishable.** Set `publish = true`; fill in `repository`, `readme`, `keywords` and
   `categories`; set `exclude` for `external/`, `tests/` and `specs/` so the package is small (the
   corpus submodule is a development resource). Do a dry run with `cargo publish --dry-run`. Publishing
   needs the `tree-sitter-gram` dependency from crates.io (0.6.2 is there).
7. **Update the documents that mention the pin**: the README's build note, `CLAUDE.md`, research R1
   and R2 in `specs/001-phase1-load-export/`, and the comment in `Cargo.toml`. Mark R1's "git
   dependency" decision as superseded.
8. **Drop what the pin needed**: the "Let cargo fetch the Relate repository" step in `.github/workflows/ci.yml`
   and the `RELATE_READ_TOKEN` secret (CI needs no credential once the dependency comes from crates.io),
   and any `[patch]` override a developer used for a local Relate checkout.

If the release's API differs from the pinned commit, fix that first with a failing test, as with any
change, and record the difference in a spike note so the ADRs that cite the old behavior stay honest.
