# Releasing relate-gram

## Launch day

Done: Relate and this repository are public, and this crate builds against the published Relate 1.0.1.
Releases 0.1.0, 0.2.0 and 0.3.0 followed the steps below.

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

`relate-gram` 0.1.0, 0.2.0 and 0.3.0 are on crates.io. The `publish` workflow uploads the crate with the repository
secret `CARGO_REGISTRY_TOKEN`, and it is run by hand after the `release` workflow has created the GitHub release:
Actions, publish, Run workflow, turn "dry run" off. (It also has a `release: published` trigger, guarded by the
repository variable `CRATES_IO_PUBLISH`, but a release that the `release` workflow creates with `GITHUB_TOKEN`
does not start other workflows, so that trigger does not fire for these releases; 0.2.0 and 0.3.0 were published by hand.)
To rehearse, leave "dry run" on. The tag, `Cargo.toml` version and changelog must agree first (the `release`
workflow checks this). A published version cannot be replaced, only yanked. After publishing, check that
`https://crates.io/crates/relate-gram` shows the version.

When a new Relate minor version is released, re-run the tests and the performance check, then widen the
`~1.0` requirement in `Cargo.toml` deliberately.
