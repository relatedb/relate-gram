use super::common::*;

#[test]
fn dump_graph_reads_nodes_and_relationships() {
    let (_db, mut c) = fresh_db();
    run(&mut c, "CREATE (:A {k: 1})-[:R {w: 2.5}]->(:B:C)");
    let d = dump_graph(&mut c);
    assert_eq!(
        d.nodes,
        vec![":A {\"k\": 1}".to_string(), ":B:C {}".to_string()]
    );
    assert_eq!(
        d.relationships,
        vec!["(:A {\"k\": 1}) -[R {\"w\": 2.5}]-> (:B:C {})".to_string()]
    );
}
