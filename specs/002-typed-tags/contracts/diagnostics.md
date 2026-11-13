# Diagnostics contract changes: Typed Tags

Additions and changes to `specs/001-phase1-load-export/contracts/diagnostics.md`; the other codes are unchanged.

| Code | Severity | Meaning | Source |
|---|---|---|---|
| `value.tag-content` | error | the content of a recognized tag breaks its form or a range, or the database rejects it at load; the message names the tag and the fault | ADR-011 §2.3 |
| `value.array-mixed` | error | already exists; now also reported when an array mixes a recognized tag with plain strings, or two recognized tags | ADR-011 §2.4 |
| `export.value` | error | already exists; vectors, mixed-kind lists and the runtime values of ADR-011 §2.1 still use it; empty lists and typed values no longer do | ADR-006 §2.6, ADR-011 §2.6 |
| `value.tag` | error | unchanged: an unrecognized tag when `tags` is `error` | ADR-006 §2.3 |
| `value.tag-dropped` | warning | unchanged: an unrecognized tag that is dropped | ADR-006 §2.3 |

`value.tag-content` carries the tagged string's span. It is added to `code::ALL`, to `docs/supported-gram.md`, and gets no fix (ADR-010 §4): `fix` annotates it like other errors without a fix.
