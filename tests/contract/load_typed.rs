//! Typed tags (ADR-011 sections 2.2 and 2.3), specs/002-typed-tags User Story 1.
use super::common::*;
use relate::Value;
use relate_gram::diagnostics::code;
use relate_gram::settings::{LoadSettings, TagPolicy};

fn text(v: &Value) -> String {
    match v {
        Value::String(s) => s.to_string(),
        other => panic!("expected a string, got {other:?}"),
    }
}

/// `(valueType, toString)` of property `key` of the first node that has it.
fn node_prop_type(conn: &mut relate::Connection, key: &str) -> (String, String) {
    let q = format!(
        "MATCH (n) WHERE n.`{key}` IS NOT NULL RETURN valueType(n.`{key}`), toString(n.`{key}`) LIMIT 1"
    );
    let rows = run(conn, &q);
    assert_eq!(rows.len(), 1, "a node with property {key}");
    (text(&rows[0][0]), text(&rows[0][1]))
}

fn rel_prop_type(conn: &mut relate::Connection, key: &str) -> (String, String) {
    let q = format!(
        "MATCH ()-[r]->() WHERE r.`{key}` IS NOT NULL RETURN valueType(r.`{key}`), toString(r.`{key}`) LIMIT 1"
    );
    let rows = run(conn, &q);
    assert_eq!(rows.len(), 1, "a relationship with property {key}");
    (text(&rows[0][0]), text(&rows[0][1]))
}

#[test]
fn each_tag_loads_as_its_relate_type() {
    let (_db, mut c) = fresh_db();
    load_fixture(&mut c, "valid", "typed-all").unwrap();
    for (key, ty) in [
        ("d", "DATE NOT NULL"),
        ("t", "ZONED TIME NOT NULL"),
        ("lt", "LOCAL TIME NOT NULL"),
        ("dt", "ZONED DATETIME NOT NULL"),
        ("ldt", "LOCAL DATETIME NOT NULL"),
        ("du", "DURATION NOT NULL"),
        ("p", "POINT NOT NULL"),
        ("u", "UUID NOT NULL"),
    ] {
        assert_eq!(node_prop_type(&mut c, key).0, ty, "{key}");
    }
}

#[test]
fn typed_values_equal_the_text_they_were_written_as() {
    let (_db, mut c) = fresh_db();
    load_fixture(&mut c, "valid", "typed-all").unwrap();
    for (key, want) in [
        ("d", "2024-01-15"),
        ("t", "10:30:00Z"),
        ("lt", "10:30:00"),
        ("ldt", "2024-01-15T10:30:00"),
        ("du", "P1DT2H"),
        ("u", "123e4567-e89b-12d3-a456-426614174000"),
    ] {
        assert_eq!(node_prop_type(&mut c, key).1, want, "{key}");
    }
}

#[test]
fn a_datetime_keeps_its_zone_name_and_offset() {
    let (_db, mut c) = fresh_db();
    load_fixture(&mut c, "valid", "typed-datetime").unwrap();
    assert_eq!(
        node_prop_type(&mut c, "z").1,
        "2024-01-15T10:30:00+01:00[Europe/Paris]"
    );
    assert_eq!(node_prop_type(&mut c, "t").1, "2024-01-15T10:30:00+01:00");
    assert_eq!(
        node_prop_type(&mut c, "n").1,
        "2024-01-15T10:30:00.123456789Z"
    );
}

#[test]
fn durations_keep_their_signs_and_nanoseconds() {
    let (_db, mut c) = fresh_db();
    load_fixture(&mut c, "valid", "typed-duration").unwrap();
    assert_eq!(node_prop_type(&mut c, "n").1, "P-1DT-2H");
    assert_eq!(node_prop_type(&mut c, "x").1, "P1DT-2H");
    assert_eq!(node_prop_type(&mut c, "s").1, "PT0.123456789S");
    assert_eq!(node_prop_type(&mut c, "m").1, "P1Y2M3DT4H5M6.789S");
}

#[test]
fn points_keep_their_srid_and_coordinates() {
    let (_db, mut c) = fresh_db();
    load_fixture(&mut c, "valid", "typed-point").unwrap();
    let rows = run(
        &mut c,
        "MATCH (n) RETURN n.g.srid, n.g.x, n.g.y, n.c.srid, n.c.x, n.c.y, n.c.z",
    );
    let r = &rows[0];
    assert_eq!(fmt_value(&r[0]), "4326");
    assert_eq!(fmt_value(&r[1]), "-74.0");
    assert_eq!(fmt_value(&r[2]), "40.7");
    assert_eq!(fmt_value(&r[3]), "9157");
    assert_eq!(fmt_value(&r[6]), "3.0");
}

#[test]
fn a_fenced_tagged_string_is_read_like_the_backtick_form() {
    let (_db, mut c) = fresh_db();
    load_fixture(&mut c, "valid", "typed-fenced").unwrap();
    assert_eq!(node_prop_type(&mut c, "f").0, "DATE NOT NULL");
    assert_eq!(node_prop_type(&mut c, "f").1, "2024-01-15");
    assert_eq!(node_prop_type(&mut c, "b").1, "2024-01-16");
}

#[test]
fn relationship_properties_are_typed_too() {
    let (_db, mut c) = fresh_db();
    load_fixture(&mut c, "valid", "typed-all").unwrap();
    assert_eq!(
        rel_prop_type(&mut c, "d"),
        ("DATE NOT NULL".into(), "2020-05-05".into())
    );
    assert_eq!(rel_prop_type(&mut c, "du").0, "DURATION NOT NULL");
    assert_eq!(rel_prop_type(&mut c, "p").0, "POINT NOT NULL");
}

#[test]
fn unrecognized_and_wrong_case_tags_load_as_strings_and_are_reported() {
    let (_db, mut c) = fresh_db();
    let r = load_str(&mut c, "(a {b: md`# x`, c: Date`2024-01-01`})").unwrap();
    assert_eq!(r.counts.tags_dropped, 2);
    assert_eq!(node_prop_type(&mut c, "b").0, "STRING NOT NULL");
    assert_eq!(node_prop_type(&mut c, "c").0, "STRING NOT NULL");
    assert!(r
        .diagnostics
        .iter()
        .any(|d| d.code == code::VALUE_TAG_DROPPED));
}

#[test]
fn the_tags_setting_governs_only_unrecognized_tags() {
    let (_db, mut c) = fresh_db();
    let settings = LoadSettings {
        tags: TagPolicy::Error,
        ..LoadSettings::default()
    };
    let f = relate_gram::load(&mut c, b"(a {b: md`x`})", &settings).unwrap_err();
    assert!(f.diagnostics.iter().any(|d| d.code == code::VALUE_TAG));
    relate_gram::load(&mut c, b"(a {d: date`2024-01-15`})", &settings).unwrap();
    assert_eq!(node_prop_type(&mut c, "d").0, "DATE NOT NULL");
}

#[test]
fn a_typed_tag_in_the_top_level_record_is_validated_and_not_stored() {
    let (_db, mut c) = fresh_db();
    let r = load_str(&mut c, "{when: date`2024-01-15`}\n(a)\n").unwrap();
    assert!(r
        .diagnostics
        .iter()
        .any(|d| d.code == code::FILE_RECORD_NOT_STORED));
    assert_eq!(
        run(&mut c, "MATCH (n) RETURN count(n)")[0][0],
        Value::Int(1)
    );
    let f = load_str(&mut c, "{when: date`2024-13-45`}\n(b)\n").unwrap_err();
    assert!(f
        .diagnostics
        .iter()
        .any(|d| d.code == code::VALUE_TAG_CONTENT));
}

#[test]
fn reloading_overwrites_a_typed_value_and_a_plain_string_overwrites_it_back() {
    let (_db, mut c) = fresh_db();
    load_str(&mut c, "(x {d: date`2024-01-01`})").unwrap();
    assert_eq!(node_prop_type(&mut c, "d").1, "2024-01-01");
    load_str(&mut c, "(x {d: date`2025-02-03`})").unwrap();
    assert_eq!(node_prop_type(&mut c, "d").1, "2025-02-03");
    assert_eq!(
        run(&mut c, "MATCH (n) RETURN count(n)")[0][0],
        Value::Int(1)
    );
    load_str(&mut c, "(x {d: \"plain\"})").unwrap();
    assert_eq!(
        node_prop_type(&mut c, "d"),
        ("STRING NOT NULL".into(), "plain".into())
    );
}

#[test]
fn a_typed_value_is_not_usable_as_an_identity_property() {
    let (_db, mut c) = fresh_db();
    let settings = LoadSettings {
        node_id_property: Some("d".to_string()),
        ..LoadSettings::default()
    };
    let r = relate_gram::load(&mut c, b"(a {d: date`2024-01-15`})", &settings).unwrap();
    assert!(r
        .diagnostics
        .iter()
        .any(|d| d.code == code::ID_PROPERTY_MISSING));
}
