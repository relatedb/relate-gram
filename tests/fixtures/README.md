# Test fixtures

Gram files used by the contract, position and round-trip tests.

- `valid/*.gram`: files the loader accepts.
- `invalid/*.gram`: files the loader rejects. Each has a sibling `.expect` file with one line per
  expected diagnostic: `code line column offset`, where `line` and `column` are 1-based (the
  column counts bytes from the start of the line) and `offset` is the 0-based byte offset.
  Blank lines and lines starting with `#` are ignored.
- `reload/*.gram`: pairs of files used to test loading over an existing database.

A fixture's first lines may be `//` comments describing it; comments are ignored by the loader
(ADR-007 section 2.5).
