//! Story 3 (ADR-001 section 2.7).
use super::common::*;
use relate::Value;
use relate_gram::diagnostics::code;
use std::sync::Arc;

fn s(text: &str) -> Value {
    Value::String(Arc::from(text))
}

fn count(c: &mut relate::Connection, q: &str) -> Value {
    run(c, q)[0][0].clone()
}

fn errors(f: &relate_gram::LoadFailure) -> Vec<&'static str> {
    f.diagnostics
        .iter()
        .filter(|d| d.is_error())
        .map(|d| d.code)
        .collect()
}

#[test]
fn loading_the_same_file_twice_leaves_the_graph_identical() {
    let (_db, mut c) = fresh_db();
    load_fixture(&mut c, "reload", "v1").unwrap();
    let once = dump_graph(&mut c);
    let r = load_fixture(&mut c, "reload", "v1").unwrap();
    assert_eq!(dump_graph(&mut c), once);
    assert_eq!(r.counts.nodes_created, 0);
    assert_eq!(r.counts.relationships_created, 0);
}

#[test]
fn an_edited_file_updates_matched_elements_and_creates_new_ones() {
    let (_db, mut c) = fresh_db();
    load_fixture(&mut c, "reload", "v1").unwrap();
    let r = load_fixture(&mut c, "reload", "v2").unwrap();
    assert_eq!(r.counts.nodes_created, 1);
    assert_eq!(r.counts.relationships_created, 1);
    assert_eq!(r.counts.nodes_updated, 2);
    assert_eq!(r.counts.relationships_updated, 1);

    assert_eq!(count(&mut c, "MATCH (n) RETURN count(n)"), Value::Int(3));
    // the file's value wins, an unstated property stays, labels are unioned
    assert_eq!(node_prop(&mut c, "ada", "name"), s("Ada Lovelace"));
    assert_eq!(node_prop(&mut c, "ada", "note"), s("kept"));
    let labels = run(&mut c, "MATCH (n {`@id`: 'ada'}) RETURN labels(n)");
    let mut got: Vec<String> = match &labels[0][0] {
        Value::List(l) => l.iter().map(fmt_value).collect(),
        o => panic!("{o:?}"),
    };
    got.sort();
    assert_eq!(got, vec!["\"Person\"", "\"Pioneer\""]);
    // the matched relationship takes the new value; the new one is created
    assert_eq!(
        count(&mut c, "MATCH ()-[r {`@id`: 'k'}]->() RETURN r.since"),
        Value::Int(1841)
    );
    assert_eq!(
        count(&mut c, "MATCH ()-[r]->() RETURN count(r)"),
        Value::Int(2)
    );
}

#[test]
fn a_reference_binds_to_a_stored_element_and_changes_nothing() {
    let (_db, mut c) = fresh_db();
    run(&mut c, "CREATE (:Person {`@id`: 'ada', name: 'Ada'})");
    let r = load_str(&mut c, "(ada)-[:R]->(x:X)").unwrap();
    assert_eq!(count(&mut c, "MATCH (n) RETURN count(n)"), Value::Int(2));
    assert_eq!(node_prop(&mut c, "ada", "name"), s("Ada"));
    assert_eq!(
        count(&mut c, "MATCH (:Person)-[:R]->(:X) RETURN count(*)"),
        Value::Int(1)
    );
    assert_eq!(r.counts.nodes_created, 1);
    assert_eq!(r.counts.nodes_updated, 0);
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| d.code == code::ID_REFERENCE_ONLY),
        "{:?}",
        r.diagnostics
    );
}

#[test]
fn stored_labels_are_kept_when_the_file_adds_another() {
    let (_db, mut c) = fresh_db();
    run(&mut c, "CREATE (:X {`@id`: 'a'})");
    load_str(&mut c, "(a:Y)").unwrap();
    let rows = run(&mut c, "MATCH (n {`@id`: 'a'}) RETURN labels(n)");
    let mut got: Vec<String> = match &rows[0][0] {
        Value::List(l) => l.iter().map(fmt_value).collect(),
        o => panic!("{o:?}"),
    };
    got.sort();
    assert_eq!(got, vec!["\"X\"", "\"Y\""]);
}

#[test]
fn elements_without_an_id_are_never_matched() {
    let (_db, mut c) = fresh_db();
    run(&mut c, "CREATE (:Person {name: 'Ada'})");
    load_str(&mut c, "(ada:Person {name: \"Ada\"})").unwrap();
    assert_eq!(
        count(&mut c, "MATCH (n:Person) RETURN count(n)"),
        Value::Int(2)
    );
}

#[test]
fn anonymous_elements_in_a_file_are_created_again_on_reload() {
    let (_db, mut c) = fresh_db();
    load_str(&mut c, "(:Anon)").unwrap();
    load_str(&mut c, "(:Anon)").unwrap();
    assert_eq!(
        count(&mut c, "MATCH (n:Anon) RETURN count(n)"),
        Value::Int(2)
    );
}

#[test]
fn a_different_relationship_type_fails_and_writes_nothing() {
    let (_db, mut c) = fresh_db();
    load_fixture(&mut c, "reload", "v1").unwrap();
    let before = dump_graph(&mut c);
    let f = load_fixture(&mut c, "reload", "v-type-changed").unwrap_err();
    assert_eq!(errors(&f), vec![code::MATCH_TYPE_DIFFERS]);
    assert_eq!(f.diagnostics[0].span.unwrap().line, 4);
    assert_eq!(dump_graph(&mut c), before);
    assert!(!c.in_transaction());
}

#[test]
fn different_endpoints_fail_with_the_same_code() {
    let (_db, mut c) = fresh_db();
    load_str(&mut c, "(a:N)-[r:R]->(b:N)").unwrap();
    let before = dump_graph(&mut c);
    let f = load_str(&mut c, "(a:N)\n(c:N)\n(a)-[r:R]->(c)").unwrap_err();
    assert_eq!(errors(&f), vec![code::MATCH_TYPE_DIFFERS]);
    assert_eq!(dump_graph(&mut c), before);
}

#[test]
fn an_identifier_that_is_the_other_kind_fails() {
    let (_db, mut c) = fresh_db();
    load_fixture(&mut c, "reload", "v1").unwrap();
    let before = dump_graph(&mut c);
    let f = load_fixture(&mut c, "reload", "v-kind-swapped").unwrap_err();
    assert_eq!(errors(&f), vec![code::MATCH_KIND_DIFFERS]);
    assert_eq!(dump_graph(&mut c), before);

    // and the reverse: a stored relationship named as a node
    let (_db2, mut c2) = fresh_db();
    load_str(&mut c2, "(a:N)-[r:R]->(b:N)").unwrap();
    let f = load_str(&mut c2, "(r:N)").unwrap_err();
    assert_eq!(errors(&f), vec![code::MATCH_KIND_DIFFERS]);
}

#[test]
fn several_stored_elements_with_one_id_fail_and_are_listed() {
    let (_db, mut c) = fresh_db();
    run(&mut c, "CREATE (:L {`@id`: 'a'}), (:L {`@id`: 'a'})");
    let f = load_str(&mut c, "(a:L)").unwrap_err();
    assert_eq!(errors(&f), vec![code::MATCH_DB_DUPLICATE]);
    assert_eq!(f.diagnostics[0].span.unwrap().line, 1);
    assert_eq!(count(&mut c, "MATCH (n) RETURN count(n)"), Value::Int(2));
}

#[test]
fn duplicates_the_file_does_not_touch_are_not_an_error() {
    let (_db, mut c) = fresh_db();
    run(&mut c, "CREATE (:L {`@id`: 'a'}), (:L {`@id`: 'a'})");
    load_str(&mut c, "(b:L)").unwrap();
    assert_eq!(count(&mut c, "MATCH (n) RETURN count(n)"), Value::Int(3));
}
