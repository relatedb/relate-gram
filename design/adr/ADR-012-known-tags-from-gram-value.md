---
id: ADR-012
title: Known Tags Come from gram-value
status: proposed
rfcs: [RFC-002]
created: 2026-11-14
---

# ADR-012: Known Tags Come from gram-value

## 1. Context

A tagged string such as ``date`2024-01-15` `` names a type for a piece of text. relate-gram loads eight such tags as Relate values and writes them back on export, and for that it carries its own validators: about 800 lines in `src/typed/` that check the content forms of dates, times, durations, points and UUIDs (calendar rules, leap years, offsets, fractions, SRIDs) with no database, so that `check` can find a bad value.

Those rules are not specific to Relate. `gram-value` (crate `gram-value` on crates.io, version 0.1.1) is the shared model of gram's data values, published for Rust, TypeScript, Python, Go and Swift and checked against one conformance corpus of about 150 cases. It interprets ten known tags (`date`, `time`, `localtime`, `datetime`, `localdatetime`, `duration`, `point`, `uuid`, `week`, `weekDate`) and reports `invalid-form` or `out-of-range` for bad content. Its forms are general: the offset and the seconds of a time are optional, and a point is `[SRID=n;]x y [z]`. relate-gram's forms are stricter, and one differs: relate-gram requires an offset and seconds, and writes a point as extended well-known text (`SRID=4326;POINT(1 2)`), which `gram-value` rejects. The grammar's own test corpus writes ``time`17:35` `` and ``point`30 10` ``, and relate-gram accepts neither (the changelog says so under Unreleased).

Typed tags have not been released: `v0.1.0` predates them, so their forms can still change. Two copies of the date and time rules would drift, and a gram file written for one tool would load in another with different results.

## 2. Decision

### 2.1 The dependency

relate-gram depends on the `gram-value` crate, at the 0.1 series (`gram-value = "0.1"`), bumped deliberately along with the tests. The library module `typed` keeps the eight tags Relate stores and delegates their validation to `gram_value::tags`. The constitution's Rust Constraints section, which lists the permitted dependencies, is amended to add `gram-value` (a minor version of the constitution).

### 2.2 Validation is gram-value's interpretation plus a Relate profile

`Tag::validate` calls `gram_value::tags::interpret_tag` for the tag and content. A diagnostic from it (`invalid-form` or `out-of-range`) becomes the load error `value.tag-content`, at the tagged string's position, with gram-value's message. A value that interprets then passes the profile of 2.3, whose failures are `value.tag-content` errors too, with a message that says the general tag accepts the text and Relate does not. The interpretation's normal content string (spec/tags.md section 4 of gram-value) is the text handed to the Relate constructor, since it denotes the same value.

### 2.3 The profile

The profile is the narrowing that Relate's types need. It is applied after interpretation:

| Tag | Beyond the general form, Relate requires |
|---|---|
| `time` | seconds written; an offset |
| `localtime` | seconds written |
| `datetime` | seconds written; an offset |
| `localdatetime` | seconds written |
| `point` | an SRID that is 4326 or 7203 with two coordinates, or 4979 or 9157 with three; for 4326 and 4979 a latitude within [-90, 90] |
| `date`, `duration`, `uuid` | nothing |

A zone name is checked for form by gram-value; whether it is a known zone, and whether the offset is that zone's, is still left to Relate when the value is built (ADR-011 2.2).

### 2.4 The point form is gram-value's

A relate-gram point is written `SRID=4326;12.5 55.7`, with a third number for the three-dimensional SRIDs: the general form, narrowed by the profile. Extended well-known text is no longer accepted, and the `point` validator and the extended well-known text writer are removed. Because typed tags are unreleased, no released behavior changes. ADR-011 2.2 (the `point` row), 2.6 (export) and 2.9 are amended to match.

### 2.5 `week` and `weekDate`

`gram-value` knows these two tags; Relate has no type for them. relate-gram does not recognize them, so a string tagged `week` or `weekDate` is treated as any unrecognized tag: dropped, with a report (ADR-006 2.3).

### 2.6 Export writes tagged strings with gram-value's text rules

The exporter builds the tagged string, or the array of them, as a `gram_value::Value` and writes it with gram-value's canonical text, so content with a backtick or a backslash is escaped as in every other gram-value language. The forms of 2.3 and 2.4 are what it writes, and exporting then reloading still gives an equal value (ADR-011 2.6).

### 2.7 Not decided here

Lowering tree-sitter nodes to `gram_value::Value` and replacing `PropValue` with it is a larger change that this ADR leaves for a later one. Until then `PropValue` and the lowering in `values.rs` stay, and only the tags and the export text of typed values use `gram-value`.

## 3. Alternatives Considered

3.1. **Keep relate-gram's own validators.** Rejected: the date, time and duration rules exist twice and can drift, and the grammar's own examples (``time`17:35` ``, ``point`30 10` ``) stay unreadable here for a reason that is not Relate's.

3.2. **Keep extended well-known text for points and use gram-value for the other seven tags.** Rejected: it keeps a validator and a writer (about 130 lines) for a form no other gram tool writes, and a point written by another tool (``point`30 10` ``, now ``point`SRID=4326;30 10` `` once an SRID is added) would need converting.

3.3. **Copy gram-value's validators into this crate.** Rejected: a copy drifts the same way, without the conformance corpus that keeps the five gram-value packages in step.

3.4. **Adopt the whole value model in one step.** Rejected for now: it rewrites `values.rs`, `load.rs`, `plan.rs` and `export.rs`, and Relate's error positions would have to travel beside values that keep none. Tags are self-contained and covered by the existing tests, so they go first.

3.5. **Ask gram-value to accept extended well-known text.** Rejected: it gives one value two accepted spellings in the shared model, which every gram-value package would then have to implement and test.

## 4. Consequences

- About 650 of the 800 lines of `src/typed/` go away: dates, times, durations and UUIDs. The profile and the thin `typed` module remain. The contract, position, round-trip and corpus tests that exist for typed tags run unchanged, except where the point form changes.
- Behavior shifts in small ways that the tests will show: error messages for bad content come from gram-value, and a fraction written with trailing zeros, an offset written `+00:00` or a point number written `30.0` is loaded in its normal form (which denotes the same value).
- The profile needs to know whether seconds were written, which `gram_value::tags` does not expose, since a time without seconds interprets as seconds zero. The implementation reads that from the text; if it proves awkward, a request to gram-value for the fact is the alternative.
- A gram file for relate-gram that wrote a point as extended well-known text needs rewriting. The changelog says so under Unreleased, with the new form.
- relate-gram's behavior now depends on the 0.1 series of gram-value; a gram-value minor version that changes a form needs a deliberate bump and a look at the profile.
- The documentation (`docs/supported-gram.md`, `docs/quickstart.md`), `design/notes/2026-11-14-gram-value.md` and SPIKE-005's notes mention extended well-known text and need updating.
- Follow-up work: the specification and tasks for this change (`specs/003-gram-value-tags/`), the constitution amendment, and a later ADR for adopting `gram_value::Value` as the loader's value type (2.7).

## 5. Related

- RFC-002 (property value mapping; its question 7.3 asked for coordination on tag vocabularies, now answered by sharing one).
- ADR-011 (typed tags; amended by this ADR), ADR-006 (property value mapping; unrecognized tags), ADR-007 (the parser path, whose constitution amendment this one follows).
- `gram-value`: https://github.com/gram-data/gram-value (its ADR-004 defines the known tags and their forms; `spec/tags.md` states them).
