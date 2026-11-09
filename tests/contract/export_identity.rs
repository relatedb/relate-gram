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
fn auto_uses_the_stored_id_as_the_identifier() {
    let (_db, mut c) = fresh_db();
    load_str(&mut c, "(ada:P)\n(grace:P)\n(ada)-[k:KNOWS]->(grace)").unwrap();
    assert_eq!(
        export_str(&mut c).unwrap(),
        "(ada:P)\n(grace:P)\n\n(ada)-[k:KNOWS]->(grace)\n"
    );
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
fn generate_ignores_stored_ids() {
    let (_db, mut c) = fresh_db();
    load_str(&mut c, "(ada:P)\n(grace:P)\n(ada)-[k:KNOWS]->(grace)").unwrap();
    let text = export_with(&mut c, &settings(IdentityMode::Generate, None, None)).unwrap();
    assert_eq!(text, "(n0:P)\n(n1:P)\n\n(n0)-[r0:KNOWS]->(n1)\n");
}

#[test]
fn property_mode_uses_the_configured_property_and_keeps_it_in_the_record() {
    let (_db, mut c) = fresh_db();
    create_node(&mut c, ":P", vec![("email", str_value("x@y"))]);
    create_node(&mut c, ":P", vec![("email", str_value("z"))]);
    let text = export_with(
        &mut c,
        &settings(IdentityMode::Property, Some("email"), None),
    )
    .unwrap();
    assert_eq!(text, "(x@y:P {email: \"x@y\"})\n(z:P {email: \"z\"})\n");
}

#[test]
fn an_integer_identity_value_is_written_as_a_bare_integer() {
    let (_db, mut c) = fresh_db();
    create_node(&mut c, ":P", vec![("uid", Value::Int(42))]);
    let text = export_with(&mut c, &settings(IdentityMode::Property, Some("uid"), None)).unwrap();
    assert_eq!(text, "(42:P {uid: 42})\n");
}

#[test]
fn a_kind_with_no_setting_is_generated_as_a_whole() {
    let (_db, mut c) = fresh_db();
    create_node(&mut c, ":P", vec![("email", str_value("a"))]);
    create_node(&mut c, ":P", vec![("email", str_value("b"))]);
    run(
        &mut c,
        "MATCH (a:P {email: 'a'}), (b:P {email: 'b'}) CREATE (a)-[:R]->(b)",
    );
    let text = export_with(
        &mut c,
        &settings(IdentityMode::Property, Some("email"), None),
    )
    .unwrap();
    assert!(text.contains("(a)-[r0:R]->(b)"), "{text}");
}

#[test]
fn property_mode_ignores_the_stored_id() {
    let (_db, mut c) = fresh_db();
    create_node(
        &mut c,
        ":P",
        vec![("@id", str_value("stored")), ("email", str_value("e"))],
    );
    let text = export_with(
        &mut c,
        &settings(IdentityMode::Property, Some("email"), None),
    )
    .unwrap();
    assert_eq!(text, "(e:P {email: \"e\"})\n");
}

#[test]
fn auto_takes_the_property_for_elements_without_an_id() {
    let (_db, mut c) = fresh_db();
    create_node(&mut c, ":P", vec![("@id", str_value("a"))]);
    create_node(&mut c, ":P", vec![("email", str_value("b"))]);
    let text = export_with(&mut c, &settings(IdentityMode::Auto, Some("email"), None)).unwrap();
    assert_eq!(text, "(a:P)\n(b:P {email: \"b\"})\n");
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

#[test]
fn identifiers_are_unique_across_nodes_and_relationships() {
    let (_db, mut c) = fresh_db();
    create_node(&mut c, ":P", vec![("@id", str_value("x"))]);
    create_node(&mut c, ":P", vec![("@id", str_value("y"))]);
    run(
        &mut c,
        "MATCH (a:P {`@id`: 'x'}), (b:P {`@id`: 'y'}) CREATE (a)-[:R {`@id`: 'x'}]->(b)",
    );
    let f = export_str(&mut c).unwrap_err();
    assert_eq!(
        codes_of(&f),
        vec!["export.id-duplicate", "export.id-duplicate"]
    );
}

#[test]
fn a_stored_id_that_is_not_a_string_is_unusable() {
    let (_db, mut c) = fresh_db();
    create_node(&mut c, ":P", vec![("@id", Value::Int(5))]);
    let f = export_str(&mut c).unwrap_err();
    assert_eq!(codes_of(&f), vec!["export.id-unusable"]);
}

#[test]
fn bad_settings_are_configuration_errors() {
    let (_db, mut c) = fresh_db();
    run(&mut c, "CREATE (:P)");
    let f = export_with(&mut c, &settings(IdentityMode::Property, None, None)).unwrap_err();
    assert_eq!(codes_of(&f), vec!["export.config"]);
    let f = export_with(
        &mut c,
        &settings(IdentityMode::Generate, Some("email"), None),
    )
    .unwrap_err();
    assert_eq!(codes_of(&f), vec!["export.config"]);
}

#[test]
fn identifiers_that_need_quoting_are_quoted() {
    let (_db, mut c) = fresh_db();
    create_node(&mut c, ":P", vec![("@id", str_value("my id"))]);
    create_node(&mut c, ":P", vec![("@id", str_value("@x"))]);
    create_node(&mut c, ":P", vec![("@id", str_value("true"))]);
    create_node(&mut c, ":P", vec![("@id", str_value("007"))]);
    assert_eq!(
        export_str(&mut c).unwrap(),
        "(`my id`:P)\n(`@x`:P)\n(`true`:P)\n(`007`:P)\n"
    );
}
