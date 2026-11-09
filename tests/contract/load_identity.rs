//! Story 1, scenario 3 (ADR-001 sections 2.1 to 2.6 and 2.8, ADR-004).
use super::common::*;
use relate::Value;
use std::sync::Arc;

fn s(text: &str) -> Value {
    Value::String(Arc::from(text))
}

fn ids(conn: &mut relate::Connection) -> Vec<String> {
    run(conn, "MATCH (n) RETURN n.`@id` ORDER BY id(n)")
        .iter()
        .map(|r| fmt_value(&r[0]))
        .collect()
}

#[test]
fn identifiers_are_stored_as_text_and_equal_by_text() {
    let (_db, mut c) = fresh_db();
    load_fixture(&mut c, "valid", "identity").unwrap();
    // `42` and `` `42` `` are one node, stored as the string "42"
    assert_eq!(
        run(&mut c, "MATCH (n:Num) RETURN n.`@id`, count(n)")[0],
        vec![s("42"), Value::Int(1)]
    );
    let all = ids(&mut c);
    assert!(all.contains(&"\"42\"".to_string()));
    assert!(all.contains(&"\"my id\"".to_string()));
    assert_eq!(all.iter().filter(|x| *x == "\"42\"").count(), 1);
}

#[test]
fn anonymous_elements_have_no_id_and_are_counted() {
    let (_db, mut c) = fresh_db();
    let r = load_fixture(&mut c, "valid", "identity").unwrap();
    let none = run(&mut c, "MATCH (n) WHERE n.`@id` IS NULL RETURN count(n)");
    assert_eq!(none[0][0], Value::Int(2));
    let w = r
        .diagnostics
        .iter()
        .find(|d| d.code == "load.anonymous")
        .expect("anonymous warning");
    assert!(w.message.contains('2'), "{}", w.message);
    assert_eq!(r.counts.elements_without_identifier, 2);
}

#[test]
fn a_node_referenced_before_and_after_its_definition_is_one_node() {
    let (_db, mut c) = fresh_db();
    load_str(&mut c, "(a)-[:R]->(b)\n(a:First {k: 1})\n(b:Second)").unwrap();
    assert_eq!(
        run(&mut c, "MATCH (n) RETURN count(n)")[0][0],
        Value::Int(2)
    );
    assert_eq!(
        run(&mut c, "MATCH (n:First) RETURN n.k")[0][0],
        Value::Int(1)
    );
    assert_eq!(run(&mut c, "MATCH (n:Second) RETURN n.`@id`")[0][0], s("b"));
}

#[test]
fn a_referenced_only_node_is_created_empty_and_reported() {
    let (_db, mut c) = fresh_db();
    let r = load_fixture(&mut c, "valid", "identity").unwrap();
    let mut warned: Vec<String> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == "id.reference-only")
        .map(|d| d.message.clone())
        .collect();
    warned.sort();
    assert_eq!(warned.len(), 3, "{:?}", r.diagnostics);
    for id in ["a", "y", "z"] {
        assert!(warned.iter().any(|m| m.contains(id)), "{id} in {warned:?}");
    }
    assert_eq!(
        run(
            &mut c,
            "MATCH (n {`@id`: 'a'}) RETURN labels(n), properties(n)"
        )[0]
        .len(),
        2
    );
}

#[test]
fn relationship_identifiers_and_directions() {
    let (_db, mut c) = fresh_db();
    load_fixture(&mut c, "valid", "identity").unwrap();
    let r = run(
        &mut c,
        "MATCH (a)-[r {`@id`: 'r'}]->(b) RETURN a.`@id`, type(r), r.w, b.`@id`",
    );
    assert_eq!(r.len(), 1, "the repeated [r] is the same relationship");
    assert_eq!(r[0], vec![s("x"), s("REL"), Value::Int(1), s("y")]);
    // (z)<-[left:L]-(x) runs from x to z
    let l = run(
        &mut c,
        "MATCH (a)-[r {`@id`: 'left'}]->(b) RETURN a.`@id`, b.`@id`",
    );
    assert_eq!(l[0], vec![s("x"), s("z")]);
}

#[test]
fn anonymous_relationships_get_no_id() {
    let (_db, mut c) = fresh_db();
    load_str(&mut c, "(a)-[:R]->(b)").unwrap();
    let rows = run(&mut c, "MATCH ()-[r]->() RETURN r.`@id`");
    assert_eq!(rows[0][0], Value::Null);
}
