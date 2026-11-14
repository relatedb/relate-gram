---
id: ADR-012
title: Known Tags Come from gram-value
status: proposed
rfcs: [RFC-002]
created: 2026-11-14
---

# ADR-012: Known Tags Come from gram-value

## 1. Context

A tagged string such as ``date`2024-01-15` `` names a type for a piece of text. relate-gram loads eight such tags as Relate values and writes them back on export, and for that it carries its own validators: 794 lines in `src/typed/` (with their unit tests) that check the content forms of dates, times, durations, points and UUIDs (calendar rules, leap years, offsets, fractions, SRIDs) with no database, so that `check` can find a bad value.

Those rules are not specific to Relate. `gram-value` (crate `gram-value` on crates.io, version 0.1.1, no runtime dependencies) is the shared model of gram's data values, published for Rust, TypeScript, Python, Go and Swift and checked against one conformance corpus of about 150 cases. It interprets ten known tags (`date`, `time`, `localtime`, `datetime`, `localdatetime`, `duration`, `point`, `uuid`, `week`, `weekDate`) and reports `invalid-form` or `out-of-range` for bad content. Its forms are general, and they describe meaning, not spelling: the offset and the seconds of a time are optional, and a point is `[SRID=n;]x y [z]`.

relate-gram's forms (ADR-011 2.2) are stricter, and one differs in kind. A time needs an offset, because Relate otherwise fills in UTC without saying so (SPIKE-005: `time('10:30')` is accepted and read as UTC), so a file would load as something it did not state. A time also needs seconds, but that is only a spelling rule: `10:30` and `10:30:00` are one time and Relate reads them equally. A point is written as extended well-known text (`SRID=4326;POINT(1 2)`), which `gram-value` rejects. The grammar's own test corpus writes ``time`17:35` `` and ``point`30 10` ``, and the changelog under Unreleased says relate-gram accepts neither.

Typed tags have not been released: `v0.1.0` predates them, so their forms can still change. ADR-011 2.9 already records that the forms are a profile of `gram-value`'s known tags and that adopting the crate is a separate decision. Two copies of the date and time rules would drift, and a gram file written for one tool would load in another with different results.

## 2. Decision

### 2.1 The dependency

relate-gram depends on the `gram-value` crate at the 0.1 series (`gram-value = "0.1"`), bumped deliberately along with the tests. `gram-value` has no runtime dependencies, and its minimum Rust version (1.75) is below the toolchain pinned here. The first bullet of the constitution's Rust Constraints is amended to add "and, for the known tags, on `gram-value`", which raises the constitution by a minor version.

The library module `typed` keeps the eight tags Relate stores (`Tag::from_name` and `Tag::ALL` are unchanged) and delegates their validation to `gram_value::tags`.

### 2.2 Validation is gram-value's interpretation plus a Relate profile

`Tag::validate` calls `gram_value::tags::interpret_tag` for the tag and content. If it reports a diagnostic, the load error is `value.tag-content`, at the tagged string's position (the node that `values.rs` already reports), with gram-value's message (the tag name that gram-value prefixes is removed, since the error message names the tag). The distinction between `invalid-form` and `out-of-range` is not kept. A value that interprets then passes the profile of 2.3; its failures are `value.tag-content` errors too, with a message that says the general tag accepts the text and Relate does not.

What the loader passes to Relate comes from the interpretation. For the text tags (`date`, `time`, `localtime`, `datetime`, `localdatetime`, `duration`, `uuid`) `TypedContent::Text` holds the interpretation's normal content string (gram-value's `spec/tags.md` section 4), which denotes the same value as the text as written: seconds present, a fraction without trailing zeros, `Z` for `+00:00`, a lower-case UUID. A zone name is kept as written, so the matching of a database message to a value (`typed_failure`) still works. For `point`, the loader builds the value from the numbers, as it does now: `TypedContent::Point` stays, filled from `gram_value::tags::Point` once the profile has checked the SRID.

### 2.3 The profile

The profile is the narrowing that Relate's types need. It is applied after interpretation:

| Tag | Beyond the general form, Relate requires |
|---|---|
| `time`, `datetime` | an offset (`Z`, `+hh:mm` or `-hh:mm`) |
| `point` | an SRID that is 4326 or 7203 with two coordinates, or 4979 or 9157 with three; for 4326 and 4979 a latitude within [-90, 90] |
| `localtime`, `localdatetime`, `date`, `duration`, `uuid` | nothing (the general form of `localtime` and `localdatetime` already forbids an offset) |

Seconds are no longer required: a time or datetime without them loads as the same value with seconds zero, which is how every gram tool reads it. This amends ADR-011 2.2, which required them. The point rule rejects a point with no SRID, so the corpus's ``point`30 10` `` still does not load here; the message says the general tag accepts it and Relate needs an SRID. A zone name is checked for form by gram-value; whether it is a known zone, and whether the offset is that zone's, is still left to Relate when the value is built (ADR-011 2.2).

### 2.4 The point form is gram-value's

A relate-gram point is written `SRID=4326;12.5 55.7`, with a third number for the three-dimensional SRIDs: the general form, narrowed by the profile. Extended well-known text is no longer accepted, and the `point` validator is removed. Because typed tags are unreleased, no released behavior changes, and no fix is offered for the old spelling (ADR-010 section 4). ADR-011 2.2 (the `point` and time rows and the ISO 8601 paragraph), 2.6, 2.7 and 2.9 are amended to match; 2.7 keeps building a point from its parsed numbers and SRID, which now come from gram-value's `Point`.

### 2.5 `week` and `weekDate`

`gram-value` knows these two tags; Relate has no type for them. relate-gram does not recognize them, so a string tagged `week` or `weekDate` is handled like any unrecognized tag, by the tag policy of ADR-006 2.3 (a warning by default).

### 2.6 What export writes

A point is written in the form of 2.4, with each coordinate as the shortest decimal that reads back as the same number (`-74`, `40.7`, `1`; `-0` is written `0`), as gram-value writes a number inside a point. `float_text`, which always keeps a `.0`, stays for plain float properties. Every other typed value is written as before: the tag and Relate's own text (`toString`), which prints times with seconds and an offset, so they are inside the profile and reloading gives an equal value (ADR-011 2.6).

### 2.7 Verification before the old validators go

Before `src/typed/` is replaced, the old validators and the new path run over the typed fixtures (`tests/fixtures/valid/typed-*`, `tests/fixtures/invalid/tag-content-*`) and the accepted and rejected tables of `specs/002-typed-tags/`. Any case where accept and reject differ is listed in the specification for this change, with the reason. The differences expected from reading both are the point form (2.4), a point with no SRID (the old validator rejected it as a form error, the profile now does), the message for a coordinate in exponent form, and the seconds rule (2.3).

## 3. Alternatives Considered

3.1. **Keep relate-gram's own validators.** Rejected: the date, time and duration rules exist twice and can drift, and the shared model's tests would not cover this copy.

3.2. **Keep extended well-known text for points and use gram-value for the other seven tags.** Rejected: it keeps a validator and a writer (about 150 lines) for a form no other gram tool writes, and a point that another gram tool wrote as `SRID=4326;30 10` would have to be rewritten as extended well-known text here.

3.3. **Copy gram-value's validators into this crate.** Rejected: a copy drifts the same way, without the conformance corpus that keeps the five gram-value packages in step.

3.4. **Adopt the whole value model in this step** (lower tree-sitter nodes to `gram_value::Value` and replace `PropValue`). Rejected for now: it rewrites `values.rs`, `load.rs`, `plan.rs` and `export.rs`, and Relate's error positions would have to travel beside values that keep none. Tags are self-contained and covered by the existing tests, so they go first, and `PropValue` and the lowering in `values.rs` stay until a later ADR.

3.5. **Ask gram-value to accept extended well-known text.** Rejected: it gives one value two accepted spellings in the shared model, which every gram-value package would then implement and test.

3.6. **Ask gram-value to report whether seconds were written, and keep requiring them.** Rejected: it needs a change in five packages for a rule that only concerns spelling, and the value is the same either way.

## 4. Consequences

- About 700 of the 794 lines of `src/typed/` go away (dates, times, durations, UUIDs and points, with their unit tests). The thin `typed` module and the profile remain, with new unit tests for the profile.
- Relate-gram's behavior shifts in small ways that the tests show: the error message for bad content is gram-value's, a time or datetime without seconds now loads, and a fraction written with trailing zeros, an offset written `+00:00` or a point number written `30.0` is loaded in its normal form, which denotes the same value. Exported points lose their `.0` (`-74`, not `-74.0`).
- A gram file for relate-gram that wrote a point as extended well-known text needs rewriting. The changelog says so under Unreleased, with the new form.
- relate-gram's behavior now depends on the 0.1 series of gram-value; a gram-value minor version that changes a form needs a deliberate bump and a look at the profile.
- Tests and fixtures that name the old point form change: `tests/contract/export_typed.rs` (the point and list assertions), `tests/fixtures/valid/typed-point.gram`, `typed-all.gram` and `typed-lists.gram`, the `tag-content-point-*` fixtures under `tests/fixtures/invalid/`, and the point in `tests/perf.rs`. Fixtures with a time or datetime without seconds that expected an error are revisited.
- Documents that name the old form change: `docs/supported-gram.md` (the point row, and the `time` example of `value.tag-content`), `specs/002-typed-tags/` (data model, quickstart, research, tasks), `CHANGELOG.md` and ADR-011. SPIKE-002 and SPIKE-005 record what Relate did on the day they ran and are left as they are.
- Follow-up work: the specification and tasks for this change (`specs/003-gram-value-tags/`), the constitution amendment, and a later ADR for adopting `gram_value::Value` as the loader's value type (3.4).

## 5. Related

- RFC-002 (property value mapping): its goal 3.5, one documented rule that the loader, the exporter and later bindings share, and its questions 7.2 (canonical spelling) and 7.3 (tag vocabularies) are the ones this ADR serves.
- ADR-011 (typed tags; amended by this ADR, whose 2.9 is its starting point), ADR-006 (property value mapping; unrecognized tags), ADR-007 (the parser path, whose constitution amendment this one follows), ADR-010 (the fix model).
- `gram-value`: https://github.com/gram-data/gram-value (its ADR-004 defines the known tags and their forms; `spec/tags.md` states them).
