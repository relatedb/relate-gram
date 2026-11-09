---
id: ADR-010
title: The Fix Model and the fix Command
status: proposed
rfcs: [RFC-004]
created: 2026-11-09
---

# ADR-010: The Fix Model and the fix Command

## 1. Context

A load that rejects a file reports each problem with a stable code and the span of the text at fault, and the syntax tree of the file (ADR-007) gives the structure around that span. For a handful of codes the correction is a small, mechanical edit of that text, and a file written in a looser style needs it hundreds of times (RFC-004). The edit has to touch only the text at fault, so that the author's comments, layout and line endings survive, and the tool must not overwrite the author's file unasked.

Some corrections need nothing from the author, such as writing `:` for `::`. Others need a decision only the author has: the type of a relationship that has none, or the direction of an arrow that has none (ADR-008 explains why the loader cannot choose). The command-line tool and its conventions are in ADR-009.

## 2. Decision

### 2.1 Fixes are computed from an error and the syntax tree

A new library function proposes a fix for one diagnostic by finding the node at its span in the syntax tree. A fix is a short description and a list of edits, each replacing a byte range of the original input with new text. Diagnostics and the load and check functions do not change. Ranges include the length of a skipped byte order mark, as spans do (ADR-007 section 2.2).

### 2.2 The corrections

| Code | Correction | Needs |
|---|---|---|
| `rel.no-type` | Insert the type. A bare arrow becomes bracketed: `-->` is `-[:T]->`, `<--` is `<-[:T]-`. An arrow with brackets gets `:T` after its identifier, or right after `[` when it has none: `-[{k: 1}]->` is `-[:T {k: 1}]->`, `-[r {k: 1}]->` is `-[r:T {k: 1}]->`. | `--rel-type T` |
| `rel.undirected` | Add a head (right) or a tail (left): `--` is `-->` or `<--`; `-[:K]-` is `-[:K]->` or `<-[:K]-`. | `--direction right\|left` |
| `rel.bidirectional` | Remove one head: `<-->` is `-->` (right) or `<--` (left); `<-[:K]->` is `-[:K]->` or `<-[:K]-`. | `--direction right\|left` |
| `rel.arrow-family` | Write `-` for each `=` or `~` in the arrow's line characters (the text outside its brackets). | none |
| `construct.double-colon` | Write `:` for `::`. | none |
| `value.octal` | Replace the literal by its value in decimal: `010` is `10` (read as decimal) or `8` (read as octal). | `--octal decimal\|octal` |
| `value.array-mixed` | If the array holds only integers (decimal or hexadecimal) and decimals, write each integer as a decimal: `1` is `1.0`, `0x1F` is `31.0`. An array with any other mixture, or an integer that a 64-bit float cannot hold exactly, gets no fix. | none |
| `value.tag` | Delete the tag: `` md`x` `` is `` `x` ``, and a fenced tag after the opening fence is removed. | none (appears with `--tags error`) |

An error with no row here has no fix. `--direction right` keeps the arrow pointing from the left node to the right node as written; `left` points it the other way. A warning is never fixed, since a file that only has warnings loads (`value.tag-dropped` is fixed only through the error form).

### 2.3 Rounds

A round checks the text, collects the fixes available with the options given, sorts their edits by position, skips an edit that overlaps an earlier one, applies the rest, and starts again on the new text. It stops when a round applies nothing, or after ten rounds. A fix that exposes another (a direction, then a type) therefore completes in one run.

### 2.4 Verification

After the last round the text is checked again. If it has more errors than the input, nothing is written and the command fails. Errors that remain are reported against the fixed text, with their new positions.

### 2.5 The command

`relate-gram fix FILE` takes `--rel-type NAME`, `--direction right|left`, `--octal decimal|octal`, `--tags error` (to make tags errors, so that they are fixed), and these output options:

- **Default:** the fixed text goes to a new file next to the input, named by replacing a trailing `.gram` with `.fixed.gram` (or appending `.fixed` to any other name). An existing file of that name is never replaced; the command stops and says to use `-o` or `--force`.
- **`-o PATH`:** write to `PATH`; `-o -` writes to standard output. With standard input as `FILE`, the output goes to standard output.
- **`--in-place`:** replace the input. The original is first copied to `FILE.orig`, which is never overwritten without `--force`, and `--no-backup` skips the copy.
- **`--force`:** allow overwriting an existing output or backup.

Nothing is written when there is nothing to fix. A file with no errors prints that and exits 0. A file whose errors have no applicable fix prints which options would apply, writes nothing, and exits 1. When fixes were applied, the file is written and the summary goes to standard error: the count of each corrected code, the output path, and any errors that remain. Exit status is 0 if no error remains and 1 if one does. A write is atomic: the text goes to a temporary file that is renamed into place.

### 2.6 The library

The fixing is a public library function, `fix(source, settings, options)`, returning the fixed bytes, the applied fixes, the remaining diagnostics and the rounds, or a failure when the result would be worse. The command is a thin layer over it.

## 3. Alternatives Considered

3.1. **Attach fixes to each diagnostic.** Rejected for now: it changes a public type that every caller of load and check uses, and whether editors should receive fixes as data is open (RFC-004 question 7.2). Computing fixes from the tree leaves load and check untouched and can be moved later.

3.2. **Regular expressions over the text.** Rejected: they cannot tell an arrow from the same characters in a string or comment, or see that a relationship already has a type.

3.3. **Overwrite in place by default, with a backup.** Rejected: the default is to leave the author's file alone, with overwriting a deliberate act (RFC-004 goal 3.3).

3.4. **Write only when every error is fixed.** Rejected: a partly fixed file is progress, it is a separate file, and the remaining errors are reported against it.

3.5. **Infer a relationship type from the labels of its endpoints.** Rejected: the loader does not guess types (ADR-008), and the fixer would be guessing for it.

3.6. **Leave `::` unfixed because it can mark a type slot in other gram files.** Rejected: in a file that is being loaded as data it is an error with an obvious correction, and the summary reports the change. RFC-004 question 7.3 asks whether it should need the user's agreement.

## 4. Consequences

- The fix rules and the error rules must change together. A new rejection should say whether it has a fix; `CLAUDE.md` records this.
- Remaining errors are positioned in the fixed text, not in the original, so their line numbers can differ from an earlier `check` of the original.
- The default output, `x.fixed.gram`, matches a `*.gram` glob, so a later `check *.gram` includes it. This is intended: the fixed file should pass.
- `--octal`, `--rel-type` and `--direction` each apply to every occurrence in the file. A file that needs different types for different relationships needs hand edits for those; the fixer does not choose per relationship.
- Merging an identical duplicate definition into a reference, and renaming a reserved `@` key, are not fixed in this release (RFC-004 question 7.1).
- A byte order mark and the original line endings are kept by construction, since only edited ranges change.

## 5. Related

- RFCs: RFC-004.
- ADR-009 defines the command this extends; ADR-006 and ADR-008 define the errors it corrects.
