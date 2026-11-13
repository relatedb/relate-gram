---
id: ADR-006
title: Property Value Mapping
status: accepted
rfcs: [RFC-002]
created: 2026-11-08
---

# ADR-006: Property Value Mapping

> Amended 2026-11-13 by ADR-011 (proposed): tags that name a Cypher type give typed values, and empty lists are supported once the grammar allows them; sections 2.3, 2.5 and 2.6 and alternatives 3.2 and 3.3 are superseded where they conflict.
>
> Amended 2026-11-09: section 2.5 now says how a leading slash is written (SPIKE-003, found while implementing the exporter). tree-sitter-gram 0.6.2 fixed the parse error; the escape stays so that exports also parse with 0.6.1.

## 1. Context

A gram property value can be a string in four quoting styles, a bare symbol, an integer in decimal, hexadecimal or octal spelling, a decimal, a boolean, a range such as `1..5`, a measurement such as `5kg`, a tagged string such as ``md`# Title` ``, an array of scalars or a nested map (tree-sitter-gram's `_value` rule). Relate, the embedded Cypher database this crate loads into, stores a smaller set, and its behavior at the edges is not always what a loader needs:

- It stores 64-bit signed integers and 64-bit floats. A larger integer literal is a syntax error.
- It stores lists of one scalar type, and it also stores an empty list. A list that mixes integers and floats is silently turned into a list of floats: `[1, 2.5]` reads back `[1.0, 2.5]`. Lists of other mixed types, nested lists, maps and null elements are rejected.
- A property set to null is not stored. Repeating a key in one record keeps the last value without an error.
- It stores dates, times, durations and points, which gram has no literal for, and rejects byte arrays.
- It stores `NaN` and the infinities; gram's decimal has no spelling for them.

(All observed in SPIKE-002.) Gram's array rule is `[` followed by one or more scalars, so an empty list has no gram form, and gram's decimal is digits, a dot and digits, with no exponent. The loader must decide for each gram value what it becomes or why it is an error; the exporter must decide how each Cypher value is written, or why it cannot be. Both must follow the constitution: a value that cannot be mapped is a located error, and nothing is dropped silently except a documented, reported tag.

The parent RFC (RFC-002) proposes a strict first phase with typed tags and a later metadata phase. This ADR fixes the strict mapping for the first phase of RFC-003. It narrows the RFC in one respect: tags that name Cypher types, which the RFC places in the strict phase and which would let a date or point be written and read back, wait for a tag vocabulary that is not yet decided (RFC-002 question 7.3). Until then those values load as plain strings with a reported tag and cannot be exported. RFC-002 §5.2 and Goal 3.2 carry a note to that effect.

## 2. Decision

### 2.1 Values accepted on load

| Gram value | Cypher value |
|---|---|
| decimal integer (`42`, `-7`) | integer |
| hexadecimal integer (`0x1F`) | integer, converted (`0x1F` is 31) |
| decimal (`3.14`) | float |
| `true`, `false` | boolean |
| string in any of the four quoting styles | string, with the grammar's escapes resolved |
| bare symbol used as a value (`active`) | the string `"active"` |
| tagged string | the content as a string; the tag is dropped and reported (2.3) |
| array whose elements are all of one kind (2.2) | list |

An integer outside the signed 64-bit range is a load error that reports its position, and so is a decimal whose value is not finite as a 64-bit float, or whose nonzero digits round to `0.0`. The content of a string is the text of the grammar's `string_content` node, with escapes resolved and nothing trimmed, so a fenced string keeps its newlines. A raw newline in a single-line quoted string is a parse error (ADR-007 §2.2); a fenced string may contain newlines, and its content starts after the line break that follows the opening fence. U+0000 cannot appear in a gram source file, and no escape produces it. Other raw control characters (tab, carriage return, DEL) are kept as written. In quoted strings, a backslash followed by `b`, `f`, `n`, `r`, `t`, `/` or a backslash is an escape, as is a backslash followed by the style's own quote character (`'`, `"` or a backtick); a fenced string has no escapes. Any other backslash sequence is a parse error (ADR-007 §2.2). A hexadecimal literal has no sign, so `-0x1F` is a parse error, and a leading `+` is not accepted. A number with a leading zero and only octal digits (`010`, `00`) is an octal literal in the grammar; whether it means 8 or 10 is not settled by the grammar, and `gram-codec` reads it as 10, so it is a load error that asks for the value in decimal or hexadecimal. Other leading-zero forms (`08`, `-010`) are parse errors.

Each of these is a load error that reports its position and names the construct: a range, a measurement, a nested map as a value, and an octal literal. The grammar's array rule admits only scalars, so the parser rejects a map or array inside an array. Gram has no null, so there is no null value to map.

### 2.2 Arrays

An array loads as a list when every element is of one kind: all integers, all decimals, all booleans, or all strings. Symbols and strings of any quoting style are one kind (string), and so are tagged strings, whose tags are handled by 2.3. An array that mixes kinds is a load error that names the first element that differs, and that includes integers mixed with decimals: Relate would convert the integers to floats without saying so, which changes a stated value.

### 2.3 Tags

A tag on a tagged string is dropped and the content is stored as a string. The first phase recognizes no tag, so every tag is dropped, and every drop is reported: the load prints one warning per distinct tag with its count and first position, and the summary lists every position up to a cap with the total, so each drop is reported (Constitution II). A setting `tags` selects `warn` (the default) or `error`; the error form reports every tagged string with its position, like other load errors. There is no setting that silences the report (Constitution II). A later decision may give tags that name Cypher types, such as dates, a meaning (RFC-002 questions 7.3 and 7.4).

### 2.4 Property keys

A key is a symbol, a backtick-quoted name or a double-quoted name, and its text is the property key. A key beginning with `@` is a load error (ADR-001). A repeated key within one record is a load error that reports both positions, since the database would keep one value and discard the other. An empty key is a load error and an export error. Any other key text, including text with spaces, is accepted.

### 2.5 Values written on export

Every property value of an element is written as follows, and a value that has no entry here is an export error (2.6).

| Cypher value | Gram spelling |
|---|---|
| integer | decimal digits, with a leading `-` if negative |
| float | the shortest decimal that reads back as the same float, always with a dot and at least one fractional digit and never an exponent (`1.0`, `0.1`, `-0.0`); a very large or small float is written in full digits |
| boolean | `true`, `false` |
| string | double-quoted, with backslash, double quote, newline, carriage return, tab, backspace and form feed written as escapes |
| list of integers, floats, booleans or strings | an array of the elements written as above; at least one element |

A `/` in the leading run of blanks and slashes of a string or a quoted key is written `\/`, so a text that begins with `//` or `/**` is never written as such: in tree-sitter-gram 0.6.1 a quoted text that starts (after any blanks) with `//`, or with `/**` when a `*/` follows anywhere in the file, is read as a comment and does not parse (SPIKE-003). A slash after the first other character is written as is. The escape reads back as `/` (section 2.1).

A property key is written as a bare symbol if it matches `[A-Za-z_][0-9A-Za-z_.@-]*`, and otherwise as a backtick-quoted name in which a backtick, a backslash, a newline, a carriage return and a tab are written as `` \` ``, `\\`, `\n`, `\r` and `\t` (the same as identifiers in ADR-002). A key starting with `@` is an export error (ADR-002).

### 2.6 Values that cannot be exported

These are export errors, listed together with the element and key as ADR-002 §2.7 requires, before any output:

- a float that is `NaN`, `+Infinity` or `-Infinity`;
- an empty list, since an array needs at least one element;
- a string containing U+0000, which the grammar's text cannot carry (other control characters are written as they are);
- a key with empty text;
- a list whose elements are not all of one of the types in 2.5;
- a temporal value, a point, a vector, a map, or any other value type Relate may store.

The last item means that a database holding dates or points cannot be exported in the first phase; typed tags are the follow-up that would carry them (RFC-002 question 7.3).

### 2.7 Where the mapping applies

The same mapping applies to node properties, relationship properties and the top-level record (ADR-003), and a record value that cannot be stored is a load error. The loader and the exporter share one implementation of the tables above; language bindings call it and do not reimplement it.

## 3. Alternatives Considered

3.1. **Accept mixed integer and float arrays and store them as floats.** Rejected: Relate does this silently, so a file that says `[1, 2.5]` would be stored with the integer changed to `1.0`.

3.2. **Store tagged strings with a recognized tag as typed values now (dates, points).** Deferred: the tag vocabulary and textual forms are undecided (RFC-002 question 7.3), and storing a value of a different type than the file wrote needs that decided first.

3.3. **Write dates and points on export as tagged strings, ahead of the load side.** Rejected: a reload would drop the tag and store a string, so a round trip would change the property's type without saying so.

3.4. **Write unexportable values as strings or omit them.** Rejected: the exported file would differ from the database without notice. A listed error lets the user remove or convert the property.

3.5. **Accept ranges and measurements as strings.** Rejected: the text `1..5` or `5kg` then stands for a value that is not a string, and queries would give different answers from the source's meaning.

3.6. **Last-wins or first-wins for a repeated key.** Rejected: one stated value is discarded without a report. Relate does the same for literals, so the loader has to check.

3.7. **Add a setting that silences dropped-tag reports.** Rejected: the constitution requires each dropped tag to be reported.

3.8. **Read `010` as octal (8) or as decimal (10).** Rejected: the grammar names the token an octal literal without fixing its value, and the one parser that reads it gives 10, so either reading would disagree with another tool. An error costs the user a rewrite in decimal.

3.9. **Export floats in scientific notation.** Rejected: the grammar's decimal has no exponent, so the file would not parse.

## 4. Consequences

- Gram files that use ranges, measurements, nested maps, mixed arrays or integers beyond 64 bits do not load, and the error says which construct and where.
- A load followed by an export gives the same values, with spellings normalized: `0x1F` becomes `31`, a single-quoted string becomes double-quoted, and `active` becomes `"active"`.
- A tag on a string is lost in the first phase. Users who need it have no recourse until a later phase; the warning is the only record.
- A database containing a date, point, vector, empty list, non-finite float or NUL-containing string cannot be exported until typed tags (RFC-002 question 7.3) exist. The error lists every offender so the user can see how much of the graph is affected.
- Relate would accept an empty key; the loader does not.
- Duplicate keys in one record are an error even when the values agree, matching the single-definition rule for elements (ADR-001).
- Open and deferred: tag vocabulary and parse-failure behavior (RFC-002 questions 7.3 and 7.4), property metadata (7.7 to 7.9), and whether a gram dialect should gain an empty-array form. Escapes, fenced-string content, raw control characters, empty keys and `true` or `false` as a bare key were checked against tree-sitter-gram 0.6.1 in SPIKE-003, and are tested against its corpus as Constitution IV requires. ADR-007 decides that the loader reads the syntax tree, which is where these rules are applied.
- The `tags` setting, like the identity settings of ADR-001 and ADR-002, is specified with the CLI integration.
- `CLAUDE.md` is updated: the first phase accepts tagged strings as plain strings with a reported tag.

## 5. Related

- RFCs: RFC-002. Resolves its questions 7.1, 7.5, 7.6 and 7.10, and 7.2 for the types of the first phase; leaves 7.3, 7.4 and 7.7 to 7.9 open.
- SPIKE-002 records the Relate behavior this decision rests on; SPIKE-003 records the parser behavior.
- ADR-007 decides how the loader reads the parsed nodes.
