//! Story 1, scenarios 1, 2 and 5 (ADR-001 sections 2.6 and 2.9, ADR-007 section 2.5).
use super::common::*;

#[test]
fn people_fixture_loads_the_described_graph() {
    let (_db, mut c) = fresh_db();
    let report = load_fixture(&mut c, "valid", "people").expect("loads");
    assert!(report.diagnostics.iter().all(|d| !d.is_error()));
    let g = dump_graph(&mut c);
    let ada = ":Person {\"@id\": \"ada\", \"born\": 1815, \"name\": \"Ada\"}";
    let grace = ":Person {\"@id\": \"grace\", \"born\": 1906, \"name\": \"Grace\"}";
    let alan = ":Mathematician:Person {\"@id\": \"alan\", \"name\": \"Alan\"}";
    let mut nodes = vec![ada, grace, alan];
    nodes.sort();
    assert_eq!(g.nodes, nodes);
    let mut rels = vec![
        format!("({ada}) -[KNOWS {{\"since\": 1840}}]-> ({grace})"),
        format!("({alan}) -[KNOWS {{}}]-> ({ada})"),
        // a left arrow: (ada)<-[:MENTORS]-(grace) is a relationship from grace to ada
        format!("({grace}) -[MENTORS {{}}]-> ({ada})"),
    ];
    rels.sort();
    assert_eq!(g.relationships, rels);
}

#[test]
fn elements_are_created_in_order_of_first_appearance() {
    let (_db, mut c) = fresh_db();
    load_fixture(&mut c, "valid", "people").unwrap();
    let rows = run(&mut c, "MATCH (n) RETURN n.`@id` ORDER BY id(n)");
    let ids: Vec<String> = rows.iter().map(|r| fmt_value(&r[0])).collect();
    assert_eq!(ids, vec!["\"ada\"", "\"grace\"", "\"alan\""]);
}

#[test]
fn a_path_equals_its_relationships_written_separately() {
    let (_db, mut a) = fresh_db();
    load_str(&mut a, "(a:N)-[:R]->(b:N)-[:S]->(c:N)").unwrap();
    let (_db2, mut b) = fresh_db();
    load_str(&mut b, "(a:N)\n(b:N)\n(c:N)\n(a)-[:R]->(b)\n(b)-[:S]->(c)").unwrap();
    assert_eq!(dump_graph(&mut a), dump_graph(&mut b));
}

#[test]
fn a_path_may_mix_directions() {
    let (_db, mut c) = fresh_db();
    load_str(&mut c, "(a)-[:R]->(b)<-[:S]-(c)").unwrap();
    let rows = run(
        &mut c,
        "MATCH (x)-[r]->(y) RETURN x.`@id`, type(r), y.`@id` ORDER BY type(r)",
    );
    let got: Vec<Vec<String>> = rows
        .iter()
        .map(|r| r.iter().map(fmt_value).collect())
        .collect();
    assert_eq!(
        got,
        vec![
            vec!["\"a\"", "\"R\"", "\"b\""],
            vec!["\"c\"", "\"S\"", "\"b\""]
        ]
    );
}

#[test]
fn comments_are_ignored_and_empty_inputs_load() {
    let (_db, mut c) = fresh_db();
    let r = load_fixture(&mut c, "valid", "empty").unwrap();
    assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    assert_eq!(
        run(&mut c, "MATCH (n) RETURN count(n)")[0][0],
        relate::Value::Int(0)
    );
    let r = load_str(&mut c, "").unwrap();
    assert!(r.diagnostics.is_empty());
    let r = load_fixture(&mut c, "valid", "record-only").unwrap();
    assert!(r.diagnostics.iter().all(|d| !d.is_error()));
    assert_eq!(
        run(&mut c, "MATCH (n) RETURN count(n)")[0][0],
        relate::Value::Int(0)
    );
}

#[test]
fn a_label_and_type_with_a_backtick_space_and_at_load_unchanged() {
    let (_db, mut c) = fresh_db();
    load_str(&mut c, "(a:`my\\`label`)-[:`odd type@`]->(b:`@x`)").unwrap();
    let rows = run(
        &mut c,
        "MATCH (a)-[r]->(b) RETURN labels(a), type(r), labels(b)",
    );
    let got: Vec<String> = rows[0].iter().map(fmt_value).collect();
    assert_eq!(got, vec!["[\"my`label\"]", "\"odd type@\"", "[\"@x\"]"]);
}

#[test]
fn counts_report_what_was_created() {
    let (_db, mut c) = fresh_db();
    let r = load_fixture(&mut c, "valid", "people").unwrap();
    assert_eq!(r.counts.nodes_created, 3);
    assert_eq!(r.counts.relationships_created, 3);
}
