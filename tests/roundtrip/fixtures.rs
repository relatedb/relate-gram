//! Story 5 (Constitution IV, SC-005): load, export, and load again.
use super::common::*;
use relate::Value;
use relate_gram::settings::{ExportSettings, IdentityMode, LoadSettings};

const FIXTURES: &[&str] = &["people", "values", "identity", "empty", "record-only"];

fn ids(conn: &mut relate::Connection) -> Vec<String> {
    let mut v: Vec<String> = run(conn, "MATCH (n) WHERE n.`@id` IS NOT NULL RETURN n.`@id`")
        .iter()
        .map(|r| fmt_value(&r[0]))
        .collect();
    v.sort();
    v
}

#[test]
fn every_valid_fixture_survives_load_export_load() {
    for name in FIXTURES {
        let (_a, mut first) = fresh_db();
        load_fixture(&mut first, "valid", name).unwrap();
        let text = export_str(&mut first).unwrap();

        let (_b, mut second) = fresh_db();
        load_str(&mut second, &text)
            .unwrap_or_else(|f| panic!("{name}: reload failed: {f:?}\n{text}"));

        assert_eq!(
            dump_graph_without_ids(&mut first),
            dump_graph_without_ids(&mut second),
            "{name}: the graph changed\n{text}"
        );
        // every identifier the first load stored is still there
        let kept = ids(&mut first);
        let again = ids(&mut second);
        for id in &kept {
            assert!(again.contains(id), "{name}: {id} lost");
        }
    }
}

#[test]
fn a_fully_identified_file_round_trips_exactly() {
    let (_a, mut first) = fresh_db();
    load_fixture(&mut first, "reload", "v1").unwrap();
    let text = export_str(&mut first).unwrap();
    let (_b, mut second) = fresh_db();
    load_str(&mut second, &text).unwrap();
    assert_eq!(dump_graph(&mut first), dump_graph(&mut second));
    // and the text is stable
    assert_eq!(export_str(&mut second).unwrap(), text);
}

#[test]
fn a_database_built_by_cypher_round_trips() {
    let (_a, mut first) = fresh_db();
    run(
        &mut first,
        "CREATE (a:Person:Admin {name: 'Ada', born: 1815, tags: ['x', 'y'], score: 2.5}), \
         (b:Person {name: 'Grace', active: true}), (c:`Odd Label`), \
         (a)-[:KNOWS {since: 1843, note: 'a\"b'}]->(b), (b)-[:`TO C`]->(c)",
    );
    let text = export_str(&mut first).unwrap();
    let (_b, mut second) = fresh_db();
    load_str(&mut second, &text).unwrap();
    assert_eq!(
        dump_graph_without_ids(&mut first),
        dump_graph_without_ids(&mut second)
    );
}

#[test]
fn a_load_followed_by_an_export_is_stable_across_databases() {
    let (_a, mut one) = fresh_db();
    let (_b, mut two) = fresh_db();
    for c in [&mut one, &mut two] {
        load_fixture(c, "valid", "people").unwrap();
        load_fixture(c, "valid", "identity").unwrap();
    }
    assert_eq!(export_str(&mut one).unwrap(), export_str(&mut two).unwrap());
}

const IDENTIFIED_BY_EMAIL: &[u8] =
    b"(a:P {email: \"a@x\", n: 1})\n(b:P {email: \"b@x\", n: 2})\n(a)-[:R]->(b)";

fn by_email() -> LoadSettings {
    LoadSettings {
        node_id_property: Some("email".into()),
        ..LoadSettings::default()
    }
}

fn export_by_email() -> ExportSettings {
    ExportSettings {
        identity: IdentityMode::Property,
        node_id_property: Some("email".into()),
        relationship_id_property: None,
    }
}

#[test]
fn an_identity_property_round_trips_without_duplicating_nodes() {
    let (_a, mut c) = fresh_db();
    relate_gram::load(&mut c, IDENTIFIED_BY_EMAIL, &by_email()).unwrap();
    let text = export_with(&mut c, &export_by_email()).unwrap();
    let before = dump_graph(&mut c);
    // loading the export with the same setting matches the same nodes
    relate_gram::load(&mut c, text.as_bytes(), &by_email()).unwrap();
    assert_eq!(count_nodes(&mut c), 2);
    assert_eq!(dump_graph(&mut c).nodes, before.nodes);
}

#[test]
fn loading_a_property_mode_export_with_default_settings_duplicates_nodes() {
    // ADR-002 section 2.1: the identities do not line up.
    let (_a, mut c) = fresh_db();
    relate_gram::load(&mut c, IDENTIFIED_BY_EMAIL, &by_email()).unwrap();
    let text = export_with(&mut c, &export_by_email()).unwrap();
    load_str(&mut c, &text).unwrap();
    assert_eq!(count_nodes(&mut c), 4);
}

fn count_nodes(c: &mut relate::Connection) -> i64 {
    match run(c, "MATCH (n) RETURN count(n)")[0][0] {
        Value::Int(i) => i,
        _ => panic!(),
    }
}
