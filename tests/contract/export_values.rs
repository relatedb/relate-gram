//! Story 4 (ADR-006 sections 2.5 and 2.6).
use super::common::*;
use relate::Value;
use std::sync::Arc;

fn list(items: Vec<Value>) -> Value {
    Value::List(Arc::new(items))
}

#[test]
fn integers_booleans_strings_and_lists() {
    let (_db, mut c) = fresh_db();
    create_node(
        &mut c,
        "",
        vec![
            ("i", Value::Int(-7)),
            ("min", Value::Int(i64::MIN)),
            ("max", Value::Int(i64::MAX)),
            ("t", Value::Bool(true)),
            ("f", Value::Bool(false)),
            ("s", str_value("plain")),
            ("empty", str_value("")),
            ("ints", list(vec![Value::Int(1), Value::Int(2)])),
            ("strs", list(vec![str_value("a"), str_value("b")])),
            ("bools", list(vec![Value::Bool(true), Value::Bool(false)])),
            ("floats", list(vec![Value::Float(1.5), Value::Float(2.0)])),
        ],
    );
    assert_eq!(
        export_str(&mut c).unwrap(),
        "(n0 {bools: [true, false], empty: \"\", f: false, floats: [1.5, 2.0], i: -7, \
         ints: [1, 2], max: 9223372036854775807, min: -9223372036854775808, \
         s: \"plain\", strs: [\"a\", \"b\"], t: true})\n"
    );
}

#[test]
fn strings_are_double_quoted_with_the_escapes() {
    let (_db, mut c) = fresh_db();
    create_node(
        &mut c,
        "",
        vec![("s", str_value("a\"b\\c\nd\re\tf\u{8}g\u{c}h it's `x`"))],
    );
    assert_eq!(
        export_str(&mut c).unwrap(),
        "(n0 {s: \"a\\\"b\\\\c\\nd\\re\\tf\\bg\\fh it's `x`\"})\n"
    );
}

#[test]
fn floats_have_a_dot_and_no_exponent_and_read_back_as_the_same_value() {
    let values = [
        0.1,
        1.0,
        -0.0,
        2.5,
        -123456.789,
        1e300,
        1e-300,
        f64::MAX,
        f64::MIN_POSITIVE,
        5e-324,
        1.7976931348623157e308,
        0.30000000000000004,
    ];
    for v in values {
        let (_db, mut c) = fresh_db();
        create_node(&mut c, "", vec![("f", Value::Float(v))]);
        let text = export_str(&mut c).unwrap();
        let literal = text
            .trim()
            .trim_start_matches("(n0 {f: ")
            .trim_end_matches("})")
            .to_string();
        assert!(literal.contains('.'), "{literal}");
        assert!(!literal.contains(['e', 'E']), "{literal}");
        let (_db2, mut c2) = fresh_db();
        load_str(&mut c2, &text).unwrap();
        let rows = run(&mut c2, "MATCH (n) RETURN n.f");
        match &rows[0][0] {
            Value::Float(got) => {
                assert_eq!(got.to_bits(), v.to_bits(), "{v} written as {literal}")
            }
            o => panic!("{o:?}"),
        }
    }
    let (_db, mut c) = fresh_db();
    create_node(
        &mut c,
        "",
        vec![
            ("a", Value::Float(0.1)),
            ("b", Value::Float(1.0)),
            ("c", Value::Float(-0.0)),
        ],
    );
    assert_eq!(
        export_str(&mut c).unwrap(),
        "(n0 {a: 0.1, b: 1.0, c: -0.0})\n"
    );
}

#[test]
fn keys_that_need_quoting_are_backticked() {
    let (_db, mut c) = fresh_db();
    create_node(
        &mut c,
        "",
        vec![("my key", Value::Int(1)), ("plain", Value::Int(2))],
    );
    assert_eq!(
        export_str(&mut c).unwrap(),
        "(n0 {`my key`: 1, plain: 2})\n"
    );
}

#[test]
fn every_value_with_no_gram_form_is_listed_before_any_output() {
    let (_db, mut c) = fresh_db();
    create_node(
        &mut c,
        ":Bad",
        vec![
            ("nan", Value::Float(f64::NAN)),
            ("inf", Value::Float(f64::INFINITY)),
            ("ninf", Value::Float(f64::NEG_INFINITY)),
            ("empty list", list(vec![])),
            ("nul", str_value("a\0b")),
            ("", Value::Int(1)),
            ("@x", Value::Int(1)),
        ],
    );
    run(
        &mut c,
        "CREATE (:Bad {when: date('2020-01-01'), where: point({x: 1.0, y: 2.0})})",
    );
    let f = export_str(&mut c).unwrap_err();
    let mut got = codes_of(&f);
    got.sort();
    // nan, inf, ninf, empty list, nul, empty key, date, point
    let mut want = vec!["export.value"; 8];
    want.push("export.key-reserved");
    want.sort();
    assert_eq!(got, want, "{:#?}", f.diagnostics);
    for d in &f.diagnostics {
        assert!(d.message.contains("node "), "{}", d.message);
    }
    assert!(f.diagnostics.iter().any(|d| d.message.contains("when")));
    assert!(f.diagnostics.iter().any(|d| d.message.contains("where")));
}

#[test]
fn relationship_values_are_checked_too() {
    let (_db, mut c) = fresh_db();
    run(&mut c, "CREATE (:A)-[:R {when: date('2020-01-01')}]->(:B)");
    let f = export_str(&mut c).unwrap_err();
    assert_eq!(codes_of(&f), vec!["export.value"]);
    assert!(
        f.diagnostics[0].message.contains("relationship "),
        "{}",
        f.diagnostics[0].message
    );
}
