# Releasing relate-gram

## Launch day (Wednesday 2026-11-11)

1. Relate's repository goes public first. `relatedb/relate-gram` goes public right after (Settings, General,
   Danger Zone, Change visibility), so that release downloads and `cargo install --git` work for everyone.
2. Release 0.1.0 as below. The version and changelog are already set.
3. Once the `relate` crate is published to crates.io (the owner of the name is donating it to the
   project), follow "After Relate's public release".

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

## While Relate is not public

Builds clone `relatedb/relate` (internal) through the organization secret `RELATE_READ_TOKEN`; the
workflow step is skipped when the secret is absent. A release built this way has no other dependency on
that access: the binary is self-contained.

## After Relate's public release

Follow `design/notes/2026-11-09-launch-week-and-relate-release.md` to replace the git dependency with the
published `relate` crate, then to publish relate-gram itself:

1. In `Cargo.toml`, remove `publish = false`, and check the metadata (`repository`, `readme`, `keywords`,
   `categories`, `exclude`).
2. Run `cargo publish --dry-run --locked` locally, and check `cargo package --list` holds only what users
   need (`src/`, `docs/`, `README.md`, `LICENSE`, `CHANGELOG.md`).
3. Add the crates.io token as the repository secret `CARGO_REGISTRY_TOKEN`.
4. Try the `publish` workflow by hand with "dry run" on.
5. Set the repository variable `CRATES_IO_PUBLISH` to `true`. From then on, publishing a GitHub release
   also publishes the crate. Until the variable is set, the workflow only runs by hand.

The `relate` crate name on crates.io is being donated to the project by its current owner, so the
dependency name stays `relate`. If the name `relate-gram` needs to change, do it before the first publish:
a published name cannot be reused.
