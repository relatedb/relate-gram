//! String values are written as gram-value's canonical text and read back equal
//! (ADR-013 sections 2.5 and 2.6), including text that begins with a slash.
use super::common::*;
use relate::Value;
use std::sync::Arc;

fn s(text: &str) -> Value {
    Value::String(Arc::from(text))
}

/// Export a node holding `text` as property `k`, check the text of the property, load the export
/// into a fresh database and return the reloaded value.
fn round_trip(text: &str, want_text: &str) -> Value {
    let (_db, mut c) = fresh_db();
    create_node(&mut c, ":P", vec![("k", s(text))]);
    let out = export_str(&mut c).unwrap();
    assert!(
        out.contains(&format!("k: {want_text}")),
        "text {text:?}: wanted `k: {want_text}` in {out}"
    );
    let (_db2, mut c2) = fresh_db();
    load_str(&mut c2, &out).unwrap_or_else(|e| panic!("{text:?}: {e:?} in {out}"));
    let rows = run(&mut c2, "MATCH (n:P) RETURN n.k");
    rows[0][0].clone()
}

#[test]
fn symbols_are_bare_and_other_strings_are_quoted() {
    for (text, want) in [
        ("active", "active"),
        ("a-b", "a-b"),
        ("foo.bar", "foo.bar"),
        ("e5", "e5"),
        ("a-1.5", "a-1.5"),
        ("x1", "x1"),
        ("_", "_"),
        ("Ada Lovelace", "\"Ada Lovelace\""),
        ("true", "\"true\""),
        ("false", "\"false\""),
        ("", "\"\""),
        ("42", "\"42\""),
        ("4x", "\"4x\""),
        ("café", "\"café\""),
        ("a\"b", "\"a\\\"b\""),
        ("a\\b", "\"a\\\\b\""),
        ("a\nb", "\"a\\nb\""),
        ("a\tb", "\"a\\tb\""),
        ("a\u{8}b", "\"a\\bb\""),
        ("a\u{c}b", "\"a\\fb\""),
    ] {
        assert_eq!(round_trip(text, want), s(text), "{text:?}");
    }
}

#[test]
fn text_that_begins_with_a_slash_round_trips_without_an_escape() {
    for text in [
        "//x",
        "// not a comment",
        "///",
        "/**x",
        "/** x */ y",
        "/x",
        "  /x",
        " //x",
        "/ /x",
        "a/b",
        "http://x",
    ] {
        let want = format!("\"{text}\"");
        let back = round_trip(text, &want);
        assert_eq!(back, s(text), "{text:?}");
    }
}

#[test]
fn names_that_begin_with_a_slash_keep_their_escape_and_round_trip() {
    for name in ["//x", "/**x", "/x"] {
        let (_db, mut c) = fresh_db();
        create_node(&mut c, &format!(":`{name}`"), vec![(name, Value::Int(1))]);
        let out = export_str(&mut c).unwrap();
        assert!(out.contains("\\/"), "{name}: {out}");
        let (_db2, mut c2) = fresh_db();
        load_str(&mut c2, &out).unwrap_or_else(|e| panic!("{name}: {e:?} in {out}"));
        let rows = run(&mut c2, &format!("MATCH (n:`{name}`) RETURN n.`{name}`"));
        assert_eq!(rows[0][0], Value::Int(1), "{name}");
    }
}
