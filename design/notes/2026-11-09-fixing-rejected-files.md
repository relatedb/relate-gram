# Fixing rejected gram files

The loader rejects a gram file when it holds something that does not map to Cypher, with a located error (Constitution II). A developer then edits the file by hand. This note records a follow-on idea that is not scheduled: a fixer, like the `--fix` mode of a linter, that repairs or proposes repairs for the errors that make a file load-rejected. It binds no decision. If it goes ahead it needs an RFC, and the choices below become ADRs.

## Why

Every strict rule in ADR-001, ADR-006, ADR-007 and ADR-008 moves work onto the file's author. Many of the errors have an obvious or nearly obvious correction, and a message that says "write a right or left arrow" is followed by the author doing exactly that by hand across hundreds of lines. A file that was written for pattern matching (`(a)-->(b)`) can fail on every relationship.

## Kinds of fix

As in linters and compilers (clippy and `cargo fix` separate suggestions that are always right from ones that might not be), a fix is applied automatically only when it keeps the file's meaning.

- **Safe, applied with `fix`:**
  - a raw newline in a quoted string becomes the escape `\n`;
  - a hexadecimal or decimal spelling is normalized, an exponent in a decimal is expanded (`1e5` to `100000.0`) when the value is exact;
  - a mixed integer and float array gets its integers written as decimals (`[1, 2.5]` becomes `[1.0, 2.5]`), since Relate would do the same silently (ADR-006 §2.2);
  - an exact duplicate definition of one element is removed (ADR-001 §2.3);
  - a `::` before a label becomes `:`, if the author confirms it was not marking a type slot (RFC-001 7.12).
- **Suggested, applied only when chosen:** the fix changes meaning or needs a choice.
  - an untyped relationship needs a type (ADR-008 §2.3); the fixer could offer one type for all of them;
  - an undirected or bidirectional arrow needs a direction (ADR-008 §2.2);
  - an octal literal needs the author's intent, 8 or 10 (ADR-006 §2.1);
  - a repeated key needs the winning value (ADR-006 §2.4); two different definitions of one element need a merge (ADR-001 §2.3);
  - an annotation would have to be removed, which drops what it says;
  - a range, measurement, nested map or tag has to become a string, a flattened set of properties or a dropped tag.
- **No fix:** a syntax error with no clear repair, input that is not valid UTF-8, an integer beyond 64 bits.

## Design points to settle

- **Edit the text, not the graph.** A fix is an edit to the source file. Loading and exporting would discard comments (ADR-007 §2.5), layout and identifier spelling, so the fixer has to work on the syntax tree's source ranges and leave everything else as written.
- **Diagnostics that carry fixes.** Each load error would need a stable code, the source range of the offending text (start and end, not only a start position), and optionally a list of edits with an applicability level. The first load implementation can prepare for this at little cost by giving errors a code and a range from the beginning, even if nothing uses the fixes yet.
- **One fixing round may not be enough.** Fixing one error can reveal another that the first masked, so the fixer repeats until it finds nothing to apply, and then reports what is left.
- **Where it lives.** Candidates are a verb of the `relate` command-line integration (for example `relate gram fix file.gram`, with a mode that only prints the edits), a library function, and an editor integration. The command-line integration is not decided yet.
- **Preview.** A fix that rewrites many lines needs a dry run that shows the changes as a diff.
- **Not everything is fixable by rule.** A file that uses ranges and measurements for real data has a modelling problem, and the fixer's job there is to say so clearly, not to guess.

## Related

- ADR-001, ADR-006, ADR-007 and ADR-008 define the errors a fixer would target.
- RFC-001 7.11 (the first `relate` command-line integration) is where a fix verb would first be discussed.
