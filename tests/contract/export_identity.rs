//! Story 4 (ADR-002 sections 2.1 to 2.3 and 2.7).
use super::common::*;
use relate::Value;
use relate_gram::settings::{ExportSettings, IdentityMode};

fn settings(mode: IdentityMode, node: Option<&str>, rel: Option<&str>) -> ExportSettings {
    ExportSettings {
        identity: mode,
        node_id_property: node.map(str::to_string),
        relationship_id_property: rel.map(str::to_string),
    }
}

#[test]
fn auto_generates_for_elements_without_an_id_and_skips_collisions() {
    let (_db, mut c) = fresh_db();
    create_node(&mut c, ":P", vec![("@id", str_value("n0"))]);
    run(&mut c, "CREATE (:P), (:P)");
    // the node already named n0 forces the generated ones to start at n1
    assert_eq!(export_str(&mut c).unwrap(), "(n0:P)\n(n1:P)\n(n2:P)\n");
    // a relationship counter is separate, and avoids an @id in use anywhere
    create_node(&mut c, ":Q", vec![("@id", str_value("r0"))]);
    run(
        &mut c,
        "MATCH (a:P), (b:Q) WITH a, b LIMIT 1 CREATE (a)-[:R]->(b)",
    );
    let text = export_str(&mut c).unwrap();
    assert!(text.contains("-[r1:R]->"), "{text}");
}

#[test]
fn missing_unusable_and_duplicate_identity_values_are_all_listed_before_any_output() {
    let (_db, mut c) = fresh_db();
    create_node(&mut c, ":P", vec![("email", str_value("same"))]);
    create_node(&mut c, ":P", vec![("email", str_value("same"))]);
    create_node(&mut c, ":P", vec![("name", str_value("no email"))]);
    create_node(&mut c, ":P", vec![("email", Value::Float(1.5))]);
    create_node(&mut c, ":P", vec![("email", str_value(""))]);
    create_node(&mut c, ":P", vec![("email", Value::Int(7))]);
    create_node(&mut c, ":P", vec![("email", str_value("7"))]);
    let f = export_with(
        &mut c,
        &settings(IdentityMode::Property, Some("email"), None),
    )
    .unwrap_err();
    let mut got = codes_of(&f);
    got.sort();
    assert_eq!(
        got,
        vec![
            "export.id-duplicate",
            "export.id-duplicate",
            "export.id-duplicate",
            "export.id-duplicate",
            "export.id-missing",
            "export.id-unusable",
            "export.id-unusable",
        ]
    );
    assert!(f
        .diagnostics
        .iter()
        .all(|d| d.span.is_none() && d.message.contains("node ")));
}
