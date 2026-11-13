---
id: ADR-009
title: The relate-gram Command
status: accepted
rfcs: [RFC-001]
created: 2026-11-09
---

# ADR-009: The relate-gram Command

> Extended 2026-11-09 by ADR-010, which adds the `fix` command.

## 1. Context

The library can load and export, but a person who wants to try gram with Relate has to write Rust. The parent RFC wants load and export available through the `relate` command-line tool (goal 3.9), and leaves how open (question 7.11). Relate's tool hosts extensions (`relate run <name>`), but that mechanism and its contract are not yet used by any project outside Relate, and Relate has just launched. A person also needs to find out what is wrong with a file without a database: every error the loader reports carries a line, a column and a message, which are only useful if they are shown next to the source text.

Relate's tool opens a database directory (`--database PATH`). A directory written by this library opens there, and a query on it returns the loaded graph (checked by hand against Relate 0.16.1 and 1.0.1, and by `tests/cli.rs` for the library's own reopen).

## 2. Decision

### 2.1 A standalone command in this crate

The crate builds a binary, `relate-gram`, with four commands. It is independent of the `relate` tool, which remains the place to query. Integrating with `relate` (RFC-001 question 7.11) stays open and would be a later decision.

| Command | Does |
|---|---|
| `check FILE...` | Parses and plans each file as a load would, without a database, and reports every error and warning |
| `load FILE... --db PATH` | Loads each file, in order, into the Relate database at `PATH`, creating it if needed |
| `export --db PATH` | Writes the database as gram text |
| `normalize FILE` | Loads the file into a scratch in-memory database and writes it as gram text |

`FILE` may be `-` for standard input.

### 2.2 Options

Each option maps to a setting of the library (ADR-001 section 2.10, ADR-002 section 2.1, ADR-006 section 2.3, ADR-007 section 2.2): `--node-id-property`, `--relationship-id-property`, `--tags warn|error`, `--identity auto|generate|property`, `--max-errors N`. The command adds `-o/--output FILE` for the text of `export` and `normalize`, `-q/--quiet` to hide warnings, and `-h`, `-V`. The command reads no configuration file and no environment variable.

### 2.3 Output and exit status

Data goes to standard output (the exported text, or a one-line summary of a `check` or `load`). Diagnostics go to standard error, each rendered with its file, line and column, the source line, and a marker under the text at fault (the library's `render`, a plain-text format with no colour). Exit status is 0 on success, 1 when a file has errors (and, for `load`, nothing was written for that file), and 2 for a usage mistake or a problem reading or writing a file. A `load` of several files stops at the first file that fails; earlier files stay loaded, since each is its own transaction.

### 2.4 The library supports it

The library gains `check(source, settings)`, which returns a `CheckReport`, and `render(diagnostic, source, name)`. Both are part of its public interface (contracts/library-api.md).

## 3. Alternatives Considered

3.1. **Wait for a `relate` integration and ship only the library.** Rejected: the first thing a new user needs is to try a file, and the integration is undecided and depends on Relate's extension contract.

3.2. **Build it as a Relate extension (`relate run gram`).** Rejected for now: it ties launch to the extension contract and to a Relate release. A standalone command can later be wrapped by an extension without changing the library.

3.3. **Add the command to the library as an example only.** Rejected: an example is not installed or documented as a tool, and it cannot carry exit codes and options that people script against.

3.4. **Use an argument-parsing crate.** Rejected: the commands and options are parsed by about 110 lines, and a dependency would add build time for a single-purpose tool. Revisit if the surface grows.

3.5. **Colour and terminal detection in diagnostics.** Rejected for the first release: plain text is correct in pipes and logs. It can be added without changing the format.

## 4. Consequences

- The command surface (names, options, exit codes) is something people script against and needs the care of a public interface: changing it later is a compatibility decision.
- `load --db PATH` makes a database on disk the user's responsibility; a failed load leaves it unchanged, but there is no `--dry-run` and no confirmation before loading into an existing database.
- Reopening a database written by another process is covered by tests for this crate's own `load` and `export`, and by a manual check with the `relate` tool; no automated test runs the `relate` tool.
- `check` cannot report the errors that depend on what a database holds (`match.*`); `load` does.
- A fixer that repairs rejected files (design note of 2026-11-09) would add a command or option here and needs the diagnostics to carry edits.
- The binary links the same dependencies as the library, so it builds wherever the library does (it needs a C compiler for tree-sitter).

## 5. Related

- RFCs: RFC-001. Leaves its question 7.11 open (integration with the `relate` tool).
