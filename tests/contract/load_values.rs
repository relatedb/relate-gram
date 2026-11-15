//! Story 1, scenario 4 (ADR-006 sections 2.1 to 2.3).
use super::common::*;
use relate::Value;
use std::sync::Arc;

fn s(text: &str) -> Value {
    Value::String(Arc::from(text))
}
fn list(items: Vec<Value>) -> Value {
    Value::List(Arc::new(items))
}

fn loaded() -> (
    relate::Database,
    relate::Connection,
    relate_gram::LoadReport,
) {
    let (db, mut c) = fresh_db();
    let r = load_fixture(&mut c, "valid", "values").expect("values.gram loads");
    (db, c, r)
}

#[test]
fn strings_in_every_quoting_style_with_escapes_resolved() {
    let (_db, mut c, _) = loaded();
    assert_eq!(node_prop(&mut c, "v", "dq"), s("double"));
    assert_eq!(node_prop(&mut c, "v", "sq"), s("single"));
    assert_eq!(node_prop(&mut c, "v", "bt"), s("back"));
    assert_eq!(node_prop(&mut c, "v", "sym"), s("active"));
    assert_eq!(node_prop(&mut c, "v", "esc"), s("a\nb\t\"q\"\\"));
    assert_eq!(node_prop(&mut c, "v", "sqe"), s("it's"));
    assert_eq!(node_prop(&mut c, "v", "bte"), s("a`b"));
    assert_eq!(node_prop(&mut c, "v", "empty"), s(""));
}

#[test]
fn fenced_content_starts_after_the_fence_line_and_keeps_its_final_newline() {
    let (_db, mut c, _) = loaded();
    assert_eq!(node_prop(&mut c, "v", "fence"), s("line one\nline two\n"));
}

#[test]
fn fenced_content_keeps_backslashes() {
    let (_db, mut c) = fresh_db();
    load_str(&mut c, "(f {s: ```\na\\nb\n```})").unwrap();
    assert_eq!(node_prop(&mut c, "f", "s"), s("a\\nb\n"));
}

#[test]
fn numbers_booleans_and_hex() {
    let (_db, mut c, _) = loaded();
    assert_eq!(node_prop(&mut c, "v", "int"), Value::Int(42));
    assert_eq!(node_prop(&mut c, "v", "neg"), Value::Int(-7));
    assert_eq!(node_prop(&mut c, "v", "max"), Value::Int(i64::MAX));
    assert_eq!(node_prop(&mut c, "v", "min"), Value::Int(i64::MIN));
    assert_eq!(node_prop(&mut c, "v", "hex"), Value::Int(31));
    assert_eq!(node_prop(&mut c, "v", "dec"), Value::Float(2.75));
    assert_eq!(node_prop(&mut c, "v", "negdec"), Value::Float(-0.5));
    assert_eq!(node_prop(&mut c, "v", "t"), Value::Bool(true));
    assert_eq!(node_prop(&mut c, "v", "f"), Value::Bool(false));
}

#[test]
fn homogeneous_arrays_become_lists() {
    let (_db, mut c, _) = loaded();
    assert_eq!(
        node_prop(&mut c, "v", "ints"),
        list(vec![Value::Int(1), Value::Int(2), Value::Int(3)])
    );
    assert_eq!(
        node_prop(&mut c, "v", "strs"),
        list(vec![s("a"), s("b"), s("c")])
    );
    assert_eq!(
        node_prop(&mut c, "v", "bools"),
        list(vec![Value::Bool(true), Value::Bool(false)])
    );
    assert_eq!(
        node_prop(&mut c, "v", "decs"),
        list(vec![Value::Float(1.5), Value::Float(2.5)])
    );
}

#[test]
fn keys_may_be_quoted() {
    let (_db, mut c, _) = loaded();
    assert_eq!(node_prop(&mut c, "v", "key with spaces"), Value::Int(1));
    assert_eq!(node_prop(&mut c, "v", "back key"), Value::Int(2));
}

#[test]
fn a_tagged_string_loads_as_a_string_and_the_tag_is_reported() {
    let (_db, mut c, report) = loaded();
    assert_eq!(node_prop(&mut c, "v", "tagged"), s("# Title"));
    let dropped: Vec<_> = report
        .diagnostics
        .iter()
        .filter(|d| d.code == "value.tag-dropped")
        .collect();
    assert_eq!(dropped.len(), 1, "{:?}", report.diagnostics);
    assert_eq!(dropped[0].span.unwrap().line, 13);
}

#[test]
fn a_fenced_tag_is_also_reported() {
    let (_db, mut c) = fresh_db();
    let r = load_str(&mut c, "(t {s: ```md\nbody\n```})").unwrap();
    assert_eq!(node_prop(&mut c, "t", "s"), s("body\n"));
    assert!(r.diagnostics.iter().any(|d| d.code == "value.tag-dropped"));
}

#[test]
fn quoted_text_that_starts_with_slashes_loads() {
    // tree-sitter-gram 0.6.1 read such a text as a comment; 0.6.2 fixed it
    let (_db, mut c) = fresh_db();
    load_fixture(&mut c, "valid", "slashes").unwrap();
    assert_eq!(node_prop(&mut c, "s", "url"), s("//cdn.example.com/x"));
    assert_eq!(node_prop(&mut c, "s", "blank"), s(" //x"));
    assert_eq!(node_prop(&mut c, "s", "sq"), s("//y"));
    assert_eq!(node_prop(&mut c, "s", "bt"), s("//z"));
    assert_eq!(node_prop(&mut c, "s", "doc"), s("/**x"));
    assert_eq!(node_prop(&mut c, "s", "//key"), Value::Int(1));
    let labels = run(&mut c, "MATCH (n {`@id`: '//id'}) RETURN labels(n)");
    assert_eq!(fmt_value(&labels[0][0]), "[\"//Label\"]");
}

// ADR-013 section 2.2: the loader holds gram_value::Value; these pin the behavior through that change.

#[test]
fn negative_zero_integers_load_as_zero() {
    let (_db, mut c) = fresh_db();
    load_str(&mut c, "(a {i: -0, l: [-0, 1]})").unwrap();
    assert_eq!(node_prop(&mut c, "a", "i"), Value::Int(0));
    assert_eq!(
        node_prop(&mut c, "a", "l"),
        list(vec![Value::Int(0), Value::Int(1)])
    );
}

#[test]
fn a_decimal_a_float_cannot_hold_exactly_loads_as_the_nearest_float() {
    let (_db, mut c) = fresh_db();
    load_str(
        &mut c,
        "(a {d: 0.1000000000000000055511151231257827, z: -0.0, t: 1.50})",
    )
    .unwrap();
    assert_eq!(node_prop(&mut c, "a", "d"), Value::Float(0.1));
    assert_eq!(node_prop(&mut c, "a", "t"), Value::Float(1.5));
    match node_prop(&mut c, "a", "z") {
        Value::Float(f) => assert!(f == 0.0 && f.is_sign_negative()),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_decimal_that_overflows_or_underflows_is_rejected() {
    let (_db, mut c) = fresh_db();
    let huge = format!("(a {{d: 1{}.5}})", "0".repeat(400));
    let tiny = format!("(a {{d: 0.{}1}})", "0".repeat(400));
    for src in [huge, tiny] {
        let f = load_str(&mut c, &src).unwrap_err();
        assert!(
            codes(&f.diagnostics).contains(&"value.decimal"),
            "{src:.40}"
        );
    }
}

#[test]
fn an_array_that_mixes_tags_names_the_tag_in_the_message() {
    let (_db, mut c) = fresh_db();
    let f = load_str(
        &mut c,
        "(a {d: [date`2024-01-01`, datetime`2024-01-01T10:00Z`]})",
    )
    .unwrap_err();
    let d = f
        .diagnostics
        .iter()
        .find(|d| d.code == "value.array-mixed")
        .unwrap();
    assert!(
        d.message.contains("datetime") && !d.message.contains("tagged"),
        "{}",
        d.message
    );
    assert_eq!(d.span.unwrap().column, 27);
}
