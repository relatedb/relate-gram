//! Writing names as gram text and as Cypher text (ADR-002 section 2.6,
//! ADR-006 section 2.5, ADR-008 section 2.4; research R6).

fn is_symbol(text: &str) -> bool {
    let mut chars = text.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-' | '@'))
}

/// `-?(0|[1-9][0-9]*)`
fn is_integer(text: &str) -> bool {
    let digits = text.strip_prefix('-').unwrap_or(text);
    match digits.as_bytes() {
        [] => false,
        [b'0'] => true,
        [b'1'..=b'9', rest @ ..] => rest.iter().all(u8::is_ascii_digit),
        _ => false,
    }
}

fn backticked(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('`');
    for c in text.chars() {
        match c {
            '`' => out.push_str("\\`"),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('`');
    out
}

/// An element identifier as gram text: a bare integer or symbol when it
/// can be one, otherwise a backtick-quoted name. Text that starts with `@`
/// and the words `true` and `false` are quoted (ADR-002 section 2.6).
pub fn identifier(text: &str) -> String {
    if is_integer(text) || (is_symbol(text) && text != "true" && text != "false") {
        text.to_string()
    } else {
        backticked(text)
    }
}

/// A label, relationship type or property key as gram text.
pub fn name(text: &str) -> String {
    if is_symbol(text) {
        text.to_string()
    } else {
        backticked(text)
    }
}

/// A string value as a double-quoted gram string (ADR-006 section 2.5).
pub fn string_literal(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// A label or relationship type as a backtick-quoted Cypher name, with a
/// backtick doubled. Fails on a NUL, which Cypher text cannot carry.
pub fn cypher_name(text: &str) -> Result<String, NulInName> {
    if text.contains('\0') {
        return Err(NulInName);
    }
    Ok(format!("`{}`", text.replace('`', "``")))
}

/// A name contained a NUL character.
#[derive(Debug, PartialEq, Eq)]
pub struct NulInName;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifier_bare_integer() {
        assert_eq!(identifier("42"), "42");
        assert_eq!(identifier("0"), "0");
        assert_eq!(identifier("-7"), "-7");
    }

    #[test]
    fn identifier_leading_zero_is_not_an_integer() {
        assert_eq!(identifier("007"), "`007`");
        assert_eq!(identifier("-0"), "-0");
        assert_eq!(identifier("-00"), "`-00`");
    }

    #[test]
    fn identifier_bare_symbol() {
        assert_eq!(identifier("a"), "a");
        assert_eq!(identifier("_x.y-z@w"), "_x.y-z@w");
        assert_eq!(identifier("n0"), "n0");
    }

    #[test]
    fn identifier_quotes_what_a_symbol_cannot_hold() {
        assert_eq!(identifier("my id"), "`my id`");
        assert_eq!(identifier("@x"), "`@x`");
        assert_eq!(identifier("true"), "`true`");
        assert_eq!(identifier("false"), "`false`");
        assert_eq!(identifier(""), "``");
        assert_eq!(identifier("1a"), "`1a`");
    }

    #[test]
    fn identifier_escapes() {
        assert_eq!(identifier("a`b"), "`a\\`b`");
        assert_eq!(identifier("a\\b"), "`a\\\\b`");
        assert_eq!(identifier("a\nb\rc\td"), "`a\\nb\\rc\\td`");
        // other control characters are written raw (ADR-002 2.6)
        assert_eq!(identifier("a\u{1}b"), "`a\u{1}b`");
    }

    #[test]
    fn name_is_a_symbol_or_backticked() {
        assert_eq!(name("Person"), "Person");
        assert_eq!(name("KNOWS"), "KNOWS");
        assert_eq!(name("my type"), "`my type`");
        assert_eq!(name("@k"), "`@k`");
        assert_eq!(name("1x"), "`1x`");
        assert_eq!(name("a`b"), "`a\\`b`");
        assert_eq!(name("true"), "true");
    }

    #[test]
    fn string_literal_is_double_quoted_with_escapes() {
        assert_eq!(string_literal("abc"), "\"abc\"");
        assert_eq!(string_literal(""), "\"\"");
        assert_eq!(
            string_literal("a\"b\\c\nd\re\tf\u{8}g\u{c}h"),
            "\"a\\\"b\\\\c\\nd\\re\\tf\\bg\\fh\""
        );
        assert_eq!(string_literal("it's `x`"), "\"it's `x`\"");
    }

    #[test]
    fn cypher_name_doubles_backticks() {
        assert_eq!(cypher_name("Person").unwrap(), "`Person`");
        assert_eq!(cypher_name("a`b").unwrap(), "`a``b`");
        assert_eq!(cypher_name("my type").unwrap(), "`my type`");
        assert_eq!(cypher_name("").unwrap(), "``");
    }

    #[test]
    fn cypher_name_rejects_nul() {
        assert!(cypher_name("a\0b").is_err());
    }
}
