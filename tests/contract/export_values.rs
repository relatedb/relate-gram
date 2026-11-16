//! Story 4 (ADR-006 sections 2.5 and 2.6).
use super::common::*;
use relate::Value;
use std::sync::Arc;

fn list(items: Vec<Value>) -> Value {
    Value::List(Arc::new(items))
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
fn every_value_with_no_gram_form_is_listed_before_any_output() {
    let (_db, mut c) = fresh_db();
    create_node(
        &mut c,
        ":Bad",
        vec![
            ("nan", Value::Float(f64::NAN)),
            ("inf", Value::Float(f64::INFINITY)),
            ("ninf", Value::Float(f64::NEG_INFINITY)),
            ("nul", str_value("a\0b")),
            ("", Value::Int(1)),
            ("@x", Value::Int(1)),
        ],
    );
    run(
        &mut c,
        "CREATE (:Bad {embedding: vector([1, 2], 2, INTEGER16), \
         mixed: [date('2020-01-01'), datetime('2020-01-01T00:00:00Z')]})",
    );
    let f = export_str(&mut c).unwrap_err();
    let mut got = codes_of(&f);
    got.sort();
    // nan, inf, ninf, nul, empty key, a vector, a list that mixes temporal kinds
    let mut want = vec!["export.value"; 7];
    want.push("export.key-reserved");
    want.sort();
    assert_eq!(got, want, "{:#?}", f.diagnostics);
    for d in &f.diagnostics {
        assert!(d.message.contains("node "), "{}", d.message);
    }
    assert!(f
        .diagnostics
        .iter()
        .any(|d| d.message.contains("embedding")));
    assert!(f.diagnostics.iter().any(|d| d.message.contains("mixed")));
    assert!(f.diagnostics.iter().any(|d| d.message.contains("vector")));
}

#[test]
fn relationship_values_are_checked_too() {
    let (_db, mut c) = fresh_db();
    run(
        &mut c,
        "CREATE (:A)-[:R {embedding: vector([1, 2], 2, INTEGER16)}]->(:B)",
    );
    let f = export_str(&mut c).unwrap_err();
    assert_eq!(codes_of(&f), vec!["export.value"]);
    assert!(
        f.diagnostics[0].message.contains("relationship "),
        "{}",
        f.diagnostics[0].message
    );
}

#[test]
fn an_empty_list_is_written_as_empty_brackets_and_reads_back() {
    let (_db, mut c) = fresh_db();
    create_node(&mut c, ":L", vec![("xs", list(vec![]))]);
    let text = export_str(&mut c).unwrap();
    assert_eq!(text, "(n0:L {xs: []})\n");
    let (_db2, mut d) = fresh_db();
    load_str(&mut d, &text).unwrap();
    let rows = run(&mut d, "MATCH (n:L) RETURN n.xs, size(n.xs)");
    assert_eq!(fmt_value(&rows[0][0]), "[]");
    assert_eq!(fmt_value(&rows[0][1]), "0");
}

// ADR-013 section 2.5: the exporter's checks that stay, and what the canonical text writes.

#[test]
fn a_string_with_nul_fails_the_export_in_a_property_and_in_a_list() {
    let (_db, mut c) = fresh_db();
    create_node(&mut c, ":P", vec![("a", str_value("x\0y"))]);
    let f = export_str(&mut c).unwrap_err();
    assert!(codes_of(&f).contains(&"export.value"), "{:?}", codes_of(&f));
    let (_db2, mut c2) = fresh_db();
    create_node(
        &mut c2,
        ":P",
        vec![(
            "l",
            Value::List(Arc::new(vec![str_value("ok"), str_value("a\0b")])),
        )],
    );
    let f2 = export_str(&mut c2).unwrap_err();
    assert!(codes_of(&f2).contains(&"export.value"));
}

#[test]
fn floats_are_written_positionally_and_keep_negative_zero() {
    let (_db, mut c) = fresh_db();
    create_node(
        &mut c,
        ":P",
        vec![
            ("a", Value::Float(-0.0)),
            ("b", Value::Float(1e21)),
            ("c", Value::Float(5e-324)),
            ("d", Value::Float(2.0)),
        ],
    );
    let out = export_str(&mut c).unwrap();
    assert!(out.contains("a: -0.0"), "{out}");
    assert!(out.contains("b: 1000000000000000000000.0"), "{out}");
    assert!(out.contains("d: 2.0"), "{out}");
    assert!(!out.contains('e') || !out.contains("e-"), "{out}");
}
