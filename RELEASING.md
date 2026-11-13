# Releasing relate-gram

## Launch day (Wednesday 2026-11-11)

Done: Relate and this repository are public, and this crate builds against the published Relate 1.0.1.
Release 0.1.0 as below; the version and changelog are set.

## A release (binaries on GitHub)

1. Make sure `main` is green: CI passes, and `cargo test --release --test perf -- --ignored` if anything
   in load or export changed.
2. Set `version` in `Cargo.toml`, add the version's section to `CHANGELOG.md` (with its date), and commit
   both: `git commit -am "Release 0.1.0"`. Run `cargo build` so `Cargo.lock` follows.
3. Tag it and push the tag: `git tag v0.1.0 && git push origin main v0.1.0`.
4. The `release` workflow builds `relate-gram` for macOS (Apple silicon and Intel) and Linux (x86-64),
   checks that the tag matches `Cargo.toml`, and creates the GitHub release with the binaries,
   `SHA256SUMS`, and the changelog section as the notes. It fails before building if the tag and version
   differ.
5. Download one binary from the release and run `relate-gram --version` and the quickstart.

To rehearse without releasing, run the workflow by hand (Actions, release, Run workflow, leave
"dry run" on): it builds all three binaries and attaches them to the run as artifacts, and creates
nothing.

## Publishing to crates.io

Relate is public and `relate-gram` depends on the published `relate` 1.0.x crate, so the crate can be
published:

1. Run `cargo publish --dry-run --locked` locally, and check `cargo package --list` holds only what users
   need (`src/`, `docs/`, `README.md`, `LICENSE`, `CHANGELOG.md`).
2. Add the crates.io token as the repository secret `CARGO_REGISTRY_TOKEN`.
3. Try the `publish` workflow by hand with "dry run" on.
4. Set the repository variable `CRATES_IO_PUBLISH` to `true`. From then on, publishing a GitHub release
   also publishes the crate. Until the variable is set, the workflow only runs by hand.

A published name cannot be reused: settle the name `relate-gram` before the first publish.

When a new Relate minor version is released, re-run the tests and the performance check, then widen the
`~1.0` requirement in `Cargo.toml` deliberately.
