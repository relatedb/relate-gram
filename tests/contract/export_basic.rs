//! Story 4 (ADR-002 sections 2.4 and 2.5).
use super::common::*;

#[test]
fn nodes_then_a_blank_line_then_relationships() {
    let (_db, mut c) = fresh_db();
    run(
        &mut c,
        "CREATE (a:Person {name: 'Ada', born: 1815}), (b:Person {name: 'Grace'}), \
         (a)-[:KNOWS {since: 1843}]->(b)",
    );
    let text = export_str(&mut c).unwrap();
    assert_eq!(
        text,
        "(n0:Person {born: 1815, name: \"Ada\"})\n\
         (n1:Person {name: \"Grace\"})\n\
         \n\
         (n0)-[r0:KNOWS {since: 1843}]->(n1)\n"
    );
}

#[test]
fn the_empty_database_exports_nothing() {
    let (_db, mut c) = fresh_db();
    assert_eq!(export_str(&mut c).unwrap(), "");
}

#[test]
fn nodes_without_relationships_have_no_blank_line() {
    let (_db, mut c) = fresh_db();
    run(&mut c, "CREATE (:A), (:B)");
    assert_eq!(export_str(&mut c).unwrap(), "(n0:A)\n(n1:B)\n");
}

#[test]
fn a_node_with_no_labels_and_no_properties_is_just_its_identifier() {
    let (_db, mut c) = fresh_db();
    run(&mut c, "CREATE ()");
    assert_eq!(export_str(&mut c).unwrap(), "(n0)\n");
    run(&mut c, "CREATE ({k: 1})");
    assert_eq!(export_str(&mut c).unwrap(), "(n0)\n(n1 {k: 1})\n");
}

#[test]
fn labels_and_property_keys_are_sorted() {
    let (_db, mut c) = fresh_db();
    run(&mut c, "CREATE (:B:A {z: 1, a: 2, m: 3})");
    assert_eq!(export_str(&mut c).unwrap(), "(n0:A:B {a: 2, m: 3, z: 1})\n");
}

#[test]
fn elements_are_written_in_id_order() {
    let (_db, mut c) = fresh_db();
    run(&mut c, "CREATE (:First)");
    run(&mut c, "CREATE (:Second)");
    run(&mut c, "CREATE (:Third)");
    let text = export_str(&mut c).unwrap();
    assert_eq!(text, "(n0:First)\n(n1:Second)\n(n2:Third)\n");
}

#[test]
fn two_exports_of_an_unchanged_database_are_equal() {
    let (_db, mut c) = fresh_db();
    load_fixture(&mut c, "valid", "people").unwrap();
    load_fixture(&mut c, "valid", "identity").unwrap();
    let a = export_str(&mut c).unwrap();
    let b = export_str(&mut c).unwrap();
    assert_eq!(a, b);
    assert!(!a.is_empty());
}

#[test]
fn names_that_need_quoting_are_quoted_and_read_back() {
    let (_db, mut c) = fresh_db();
    run(
        &mut c,
        "CREATE (a:`my label` {`my key`: 1}), (b:`@x`), (a)-[:`odd type@`]->(b)",
    );
    let text = export_str(&mut c).unwrap();
    assert_eq!(
        text,
        "(n0:`my label` {`my key`: 1})\n(n1:`@x`)\n\n(n0)-[r0:`odd type@`]->(n1)\n"
    );
    let (_db2, mut c2) = fresh_db();
    load_str(&mut c2, &text).unwrap();
    let rows = run(
        &mut c2,
        "MATCH (a)-[r]->(b) RETURN labels(a), a.`my key`, type(r), labels(b)",
    );
    let got: Vec<String> = rows[0].iter().map(fmt_value).collect();
    assert_eq!(
        got,
        vec!["[\"my label\"]", "1", "\"odd type@\"", "[\"@x\"]"]
    );
}

#[test]
fn the_connection_must_not_be_in_a_transaction() {
    let (_db, mut c) = fresh_db();
    c.begin().unwrap();
    let f = export_str(&mut c).unwrap_err();
    assert_eq!(codes_of(&f), vec!["export.in-transaction"]);
    assert!(c.in_transaction());
    c.rollback().unwrap();
}

#[test]
fn export_does_not_modify_the_database() {
    let (_db, mut c) = fresh_db();
    load_fixture(&mut c, "valid", "people").unwrap();
    let before = dump_graph(&mut c);
    export_str(&mut c).unwrap();
    assert_eq!(dump_graph(&mut c), before);
    assert!(!c.in_transaction());
}
