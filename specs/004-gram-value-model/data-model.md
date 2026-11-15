# Data Model: gram-value as the Value Model

No stored entity changes.

- **Lowered value**: `value` (a `gram_value::Value` of kind integer, decimal, boolean, string, tagged or array of one kind), `span` (position of the value node), `items` (element spans of an array, empty otherwise).
- **Record**: `Vec<(String, Lowered)>` in order of writing; keys are checked as today (empty, reserved `@`, duplicate with the first as related).
- **Typed value**: a `Value::Tagged` with a recognized tag and gram-value's normal content; a point's numbers are obtained from that content when the value is built.
- **Tag::validate**: returns the normal content string or a fault message (ADR-012).
- **Export mapping** (ADR-013 section 2.5):

| Relate value | `gram_value::Value` | Errors |
|---|---|---|
| integer, boolean, string | same kinds | string with U+0000 |
| float | `decimal` | non-finite |
| list | `array`; `[]` when empty | nested list, mixed kinds |
| UUID | `tagged("uuid", lower-case text)` | |
| point | `tagged("point", "SRID=n;x y [z]")` | unsupported SRID, non-finite |
| temporal | `tagged(tag, Relate toString)` | list of mixed temporal kinds |
| other | | export error as ADR-006 section 2.6 |

- **Text**: each property is `names::name(key): <Value::to_text()>`; a record is `{...}` of these joined by `, ` in key order.
