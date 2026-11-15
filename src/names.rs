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

/// Write `text` with the escapes of a quoted gram text.
///
/// A `/` in the leading run of blanks and slashes is escaped as `\/`. In
/// tree-sitter-gram 0.6.1 a comment token (`//...` or `/**...*/`) can start
/// where a quoted text starts, and when it matches more than the text does it
/// wins, so a text that begins (after any blanks) with `//` or `/**` would not
/// parse (SPIKE-003). A slash after the first other character is safe.
fn escape_into(out: &mut String, text: &str, specials: &[(char, &str)]) {
    let mut leading = true;
    for c in text.chars() {
        if leading && c == '/' {
            out.push_str("\\/");
            continue;
        }
        if !c.is_whitespace() {
            leading = false;
        }
        match specials.iter().find(|(s, _)| *s == c) {
            Some((_, escape)) => out.push_str(escape),
            None => out.push(c),
        }
    }
}

fn backticked(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('`');
    escape_into(
        &mut out,
        text,
        &[
            ('`', "\\`"),
            ('\\', "\\\\"),
            ('\n', "\\n"),
            ('\r', "\\r"),
            ('\t', "\\t"),
        ],
    );
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
