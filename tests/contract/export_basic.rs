//! Story 4 (ADR-002 sections 2.4 and 2.5).
use super::common::*;

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
