---
id: RFC-004
title: Fixing Rejected Gram Files
status: review
created: 2026-11-09
adrs: [ADR-010]
---

# RFC-004: Fixing Rejected Gram Files

## 1. Summary

A loader that accepts only the gram constructs that map directly to Cypher refuses everything else, and each refusal is an error that says what is wrong and what to write instead. For a file with a few errors that is enough. For a file written in a looser style, the same one-line correction is needed hundreds of times, and doing it by hand is the most tedious part of adopting the strict subset. A fixer is a command that applies those corrections to the file itself, the way `cargo fix` or `eslint --fix` do for source code. Its organizing rule is that the file's author stays in control: it never overwrites the original unless asked, it reports every change it makes, and where a correction needs a decision, the author supplies it. This RFC sets out the problem, the goals, and the candidate approach; the technical decisions follow in an ADR.

## 2. Problem / Motivation

The strict subset (RFC-001) rejects constructs such as `(a)-->(b)` (no relationship type), `(a)--(b)` (no direction), fat and wavy arrows, `::` labels, mixed integer and decimal arrays, and octal literals. Evidence that these are the common failures:

- The tree-sitter-gram corpus has 211 cases of gram written by the grammar's own authors. 100 of them are rejected for a construct outside the subset (39 more are syntax errors that the corpus itself expects). 45 of those 100 are rejected only because of their arrows: no type, an undirected or bidirectional direction, or a fat or wavy arrow. Gram written for pattern matching uses these arrows by default.
- 49 of the 100 would load after the corrections this RFC proposes, given a relationship type and a direction.
- `docs/supported-gram.md` lists 22 rejected constructs, and 7 of them have a mechanical correction.
- `relate-gram check` on a file of 500 untyped relationships prints 500 errors with the same advice. The advice is right and nobody should have to follow it 500 times.

The cost lands at the first contact: a new user's first file is more likely to fail than to load, and the way forward is an editor session.

## 3. Goals

3.1. Apply the corrections that the loader's errors describe, to the file's text, for the errors whose correction is mechanical.

3.2. Report every change made, by error code and count, so that nothing is altered unnoticed.

3.3. Never overwrite the input file unless the user asks for that explicitly, and then keep a way back.

3.4. Where a correction needs a decision (which type a relationship has, which way an undirected arrow points), use only a decision the user states; never guess.

3.5. Change nothing outside the text being corrected: comments, layout, line endings and a byte order mark stay as they were.

3.6. Check the result before writing it, and never produce a file with more errors than the input.

3.7. Be repeatable: fixing a fixed file changes nothing.

## 4. Non-Goals

4.1. Fixing syntax errors (a missing value, an unclosed bracket). There is no mechanical repair that keeps the author's meaning.

4.2. Restructuring data: turning a range into two properties, flattening a nested map, removing annotations or subject patterns. These need the author's design.

4.3. Prompting interactively. A fixer has to be scriptable and runnable in CI.

4.4. Reformatting. Rewriting a file in canonical form is what export and `normalize` do.

4.5. Editor integration. Machine-readable fixes could serve an editor later; this RFC does not design that.

4.6. Loosening the loader. The loader stays strict; the fixer produces files that satisfy it.

## 5. Proposed Approach (high-level)

### 5.1 Fixes are text edits

A fix is a set of edits to the source text, each replacing a byte range with new text. Because only the ranges named by errors are touched, everything else is preserved exactly. Each error code either has a fix, has one when the user gives a decision, or has none.

### 5.2 Mechanical fixes and fixes that need a decision

Some corrections need nothing from the user: dropping a tag the loader would drop anyway, writing `:` for `::`, replacing a fat or wavy arrow with a thin one, writing the integers of a mixed numeric array as decimals. Others need a value only the user has: the type of an untyped relationship, the direction of an undirected or bidirectional arrow, the meaning of an octal literal. These apply only when the user states the value.

### 5.3 Rounds

A correction can reveal another: after an undirected arrow gets a direction, the relationship may still need a type. The fixer repeats check, fix and re-check until nothing applies, and reports whatever errors remain.

### 5.4 Safe output

By default the fixed text goes to a new file next to the input, or to standard output when the input is standard input or the user asks. Overwriting the input is a separate, explicit option, and it keeps a copy of the original. A fixer that writes nothing when there is nothing to fix, or when it would make things worse, never leaves the user with a file they did not ask for.

### 5.5 Surface

A `fix` command of the `relate-gram` tool and a function in the library, so that other tools can use the same corrections.

## 6. Alternatives Considered

6.1. **A `--fix` flag on `check`.** Rejected: `check` is a read-only command that people run freely, and it should stay one.

6.2. **A dry run that only prints a diff.** Rejected as the only mode: a diff of a large file cannot be reviewed in a terminal. A new file the author can open in an editor and compare with the original is reviewable at any size.

6.3. **Interactive prompts for each fix.** Rejected: not scriptable, and unusable on hundreds of identical fixes.

6.4. **A lenient load that repairs the file as it reads it.** Rejected: the loader's promise is that the database holds what the file says. A repaired load would hold something the file does not say.

6.5. **Leave it to `sed` or an editor's search and replace.** Rejected: a pattern cannot tell an arrow from the same characters inside a string or a comment, and it cannot see what the file defines (a relationship that already has a type must not get a second).

6.6. **Choose a default relationship type or direction.** Rejected: that is the invention the strict loader refuses to make, moved into the fixer.

Related: RFC-001 defines the strict subset whose rejections this RFC addresses, and the design note `design/notes/2026-11-09-fixing-rejected-files.md` records the first sketch.

## 7. Open Questions

7.1. Which further errors should get fixes: merging an identical duplicate definition into a reference, and renaming a reserved `@` key are candidates, and each needs a notion of equality or a new name.

7.2. Should the fixes be available to editors as data attached to errors, and in what format?

7.3. Should a fix that changes the meaning a little (writing `:` for `::`, which the gram schema conventions use to mark a type slot) require the user's agreement as a decision, or is reporting it enough?

## 8. ADRs

- ADR-010: The Fix Model and the fix Command
