# Data Model: Typed Tags for Gram Values

## Recognized tag

One of `date`, `time`, `localtime`, `datetime`, `localdatetime`, `duration`, `point`, `uuid`, lower case, matched exactly. Any other tag is unrecognized and behaves as before (ADR-006 section 2.3).

## Typed value

A property value that carries its tag and its validated content:

| Field | Meaning |
|---|---|
| tag | the recognized tag |
| content | the normalized text passed to the constructor, or for a point the parsed SRID and coordinates (x, y and an optional z) |
| position | the tagged string's source span, kept for errors raised later |

A `PropValue` is now an integer, a float, a boolean, a string, a typed value, or a list of one of those (a list holds one kind; a typed list holds one tag).

## Validation (per tag; the forms and ranges are ADR-011 section 2.2)

| Tag | Accepted | Rejected with `value.tag-content` |
|---|---|---|
| `date` | `YYYY-MM-DD`, year of 4 or more digits, optional `-` | week, ordinal, basic and `+year` forms; month or day out of range, including 29 February in a common year |
| `time`, `datetime` | seconds present; offset `Z` or `+hh:mm`; datetime may add `[zone]` | no offset; no seconds; second 60; a bare date as a datetime |
| `localtime`, `localdatetime` | seconds present; no offset | an offset; no seconds |
| `duration` | upper-case `P`, at least one component, sign per number, fraction on seconds only | `P` or `PT` alone; weeks; a leading `-` before `P`; fraction on other components; lower case |
| `point` | `SRID=n;POINT(x y)` for 4326 and 7203, `SRID=n;POINT Z(x y z)` for 4979 and 9157 | other SRIDs; a missing SRID; dimension and SRID disagreeing; exponent form; non-finite values; latitude outside [-90, 90] |
| `uuid` | 8-4-4-4-12 hexadecimal digits, either case | any other grouping, braces or no hyphens |

A fraction of seconds has 1 to 9 digits; more is rejected. The zone name of a datetime is not validated by `check`.

## Load write groups

A group is (element kind, key, tag, shape), where shape is the point's dimension or whether the value is a list. Rows hold the element's internal id and the value text (or the parsed point numbers).

## Export value

A typed value is written as `` tag`text` `` with the text of ADR-011 section 2.2: temporals as `toString` prints them, a point as extended well-known text (decimals without an exponent), a UUID in lower case. A list of one typed kind is an array of such strings.
