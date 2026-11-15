# Data Model: Known Tags from gram-value

No stored entity changes. These are the in-memory shapes after the change.

- **Tag** (unchanged): the eight recognized names; `Tag::from_name`, `Tag::ALL`, `Tag::name`.
- **TypedContent** (unchanged shape): `Text(String)` now holds gram-value's normal content; `Point(PointContent)` holds `srid: i64`, `x`, `y`, `z: Option<f64>` taken from gram-value's `Point`.
- **TypedValue** (unchanged): tag, content, span.
- **Profile** (new, `src/typed/profile.rs`): a function from the interpretation to `Ok(())` or a fault message. Rules, per ADR-012 section 2.3:

| Tag | Rule | Fault message wording |
|---|---|---|
| `time`, `datetime` | `offset` is present | the tag needs an offset; the general tag accepts the text and Relate does not |
| `point` | `srid` is present | needs an SRID; the general tag accepts the text and Relate does not |
| `point` | SRID in {4326, 7203} with `z` absent, or in {4979, 9157} with `z` present | the SRID and the number of coordinates disagree, or the SRID is not supported |
| `point` | latitude `y` in [-90, 90] when SRID is 4326 or 4979 | latitude out of range |
| others | none | |

- **Message**: `the content of the `<tag>` tag is not valid: <fault>`, as now; the fault is gram-value's message (prefix stripped) or the profile's.
- **Export text**: `point`SRID=<srid>;<x> <y>[ <z>]``; other typed values unchanged.
