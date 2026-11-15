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

#[test]
fn an_array_of_one_recognized_tag_loads_as_a_typed_list() {
    let (_db, mut c) = fresh_db();
    load_fixture(&mut c, "valid", "typed-lists").unwrap();
    for (key, ty) in [
        ("dates", "LIST<DATE NOT NULL> NOT NULL"),
        ("spans", "LIST<DURATION NOT NULL> NOT NULL"),
        ("places", "LIST<POINT NOT NULL> NOT NULL"),
        ("ids", "LIST<UUID NOT NULL> NOT NULL"),
        ("one", "LIST<DATE NOT NULL> NOT NULL"),
    ] {
        assert_eq!(node_prop_type(&mut c, key).0, ty, "{key}");
    }
    assert_eq!(
        node_prop_type(&mut c, "dates").1,
        "[2024-01-01, 2024-02-02]"
    );
}

#[test]
fn an_array_of_unrecognized_tags_is_a_list_of_strings_with_the_drops_reported() {
    let (_db, mut c) = fresh_db();
    let r = load_str(&mut c, "(a {xs: [md`a`, md`b`]})").unwrap();
    assert_eq!(r.counts.tags_dropped, 2);
    assert_eq!(
        node_prop_type(&mut c, "xs").0,
        "LIST<STRING NOT NULL> NOT NULL"
    );
}

#[test]
fn a_vector_tag_is_unrecognized_so_the_content_loads_as_a_string() {
    let (_db, mut c) = fresh_db();
    let r = load_str(&mut c, "(a {v: vector`FLOAT32[1.0, 2.0]`})").unwrap();
    assert_eq!(r.counts.tags_dropped, 1);
    assert_eq!(node_prop_type(&mut c, "v").0, "STRING NOT NULL");
}

// ADR-012: known tags come from gram-value, with a Relate profile.

#[test]
fn a_time_without_seconds_equals_the_time_with_seconds_zero() {
    let (_db, mut c) = fresh_db();
    load_fixture(&mut c, "valid", "typed-time-no-seconds").unwrap();
    for (key, want) in [
        ("t", "10:30:00Z"),
        ("dt", "2024-01-15T10:30:00+01:00"),
        ("lt", "10:30:00"),
        ("ldt", "2024-01-15T10:30:00"),
    ] {
        assert_eq!(node_prop_type(&mut c, key).1, want, "{key}");
    }
}

#[test]
fn spellings_of_one_value_load_equal() {
    let (_db, mut c) = fresh_db();
    load_str(
        &mut c,
        "(:E {a: time`10:30:00+00:00`, b: time`10:30Z`, c: localtime`10:30:00.50`, d: localtime`10:30:00.5`, \
         e: uuid`123E4567-E89B-12D3-A456-426614174000`, f: uuid`123e4567-e89b-12d3-a456-426614174000`})",
    )
    .unwrap();
    let rows = run(&mut c, "MATCH (n:E) RETURN n.a = n.b, n.c = n.d, n.e = n.f");
    assert_eq!(
        rows[0].iter().map(fmt_value).collect::<Vec<_>>(),
        ["true", "true", "true"]
    );
}

#[test]
fn a_time_or_datetime_without_an_offset_is_still_rejected() {
    let (_db, mut c) = fresh_db();
    for src in [
        "(a {t: time`17:35`})",
        "(a {t: datetime`2024-01-15T10:30`})",
    ] {
        let f = relate_gram::load(&mut c, src.as_bytes(), &LoadSettings::default()).unwrap_err();
        let d = f
            .diagnostics
            .iter()
            .find(|d| d.code == code::VALUE_TAG_CONTENT)
            .expect(src);
        assert!(d.message.contains("offset"), "{}", d.message);
    }
}

#[test]
fn the_message_for_bad_content_names_the_tag_once() {
    let (_db, mut c) = fresh_db();
    let f = relate_gram::load(
        &mut c,
        b"(a {d: date`2024-13-45`})",
        &LoadSettings::default(),
    )
    .unwrap_err();
    let m = &f.diagnostics[0].message;
    assert!(
        m.contains("`date`") && !m.contains("date: date") && !m.contains("`date`: date:"),
        "{m}"
    );
}

#[test]
fn points_in_the_general_form_load_and_other_forms_do_not() {
    let (_db, mut c) = fresh_db();
    load_str(
        &mut c,
        "(a {g: point`SRID=4326;12.5 55.7`, h: point`SRID=4979;1 2 3`})",
    )
    .unwrap();
    let rows = run(&mut c, "MATCH (n) RETURN n.g.x, n.g.y, n.h.z, n.h.srid");
    assert_eq!(
        rows[0].iter().map(fmt_value).collect::<Vec<_>>(),
        ["12.5", "55.7", "3.0", "4979"]
    );
    for src in [
        "(a {g: point`SRID=4326;POINT(1 2)`})",
        "(a {g: point`30 10`})",
        "(a {g: point`SRID=3857;1 2`})",
        "(a {g: point`SRID=4326;1 2 3`})",
        "(a {g: point`SRID=4326;1 95`})",
    ] {
        let f = relate_gram::load(&mut c, src.as_bytes(), &LoadSettings::default()).unwrap_err();
        assert!(
            f.diagnostics
                .iter()
                .any(|d| d.code == code::VALUE_TAG_CONTENT),
            "{src}"
        );
    }
}

#[test]
fn week_and_weekdate_are_unrecognized_tags() {
    let (_db, mut c) = fresh_db();
    let r = load_str(&mut c, "(a {w: week`2024-W05`, x: weekDate`2024-W05-1`})").unwrap();
    assert_eq!(r.counts.tags_dropped, 2);
    assert_eq!(node_prop_type(&mut c, "w").0, "STRING NOT NULL");
    let strict = LoadSettings {
        tags: TagPolicy::Error,
        ..LoadSettings::default()
    };
    let f = relate_gram::load(&mut c, b"(a {w: week`2024-W05`})", &strict).unwrap_err();
    assert!(f.diagnostics.iter().any(|d| d.code == code::VALUE_TAG));
}

#[test]
fn a_rejected_typed_list_element_is_reported_at_that_element() {
    let (_db, mut c) = fresh_db();
    let src = "(a {when: [datetime`2024-01-15T10:30+01:00[Europe/Paris]`, datetime`2024-01-15T10:30+01:00[No/Such_Zone]`]})";
    let f = relate_gram::load(&mut c, src.as_bytes(), &LoadSettings::default()).unwrap_err();
    let d = f.diagnostics.iter().find(|d| d.code == code::VALUE_TAG_CONTENT).unwrap();
    assert_eq!(d.span.unwrap().column, src.find("datetime`2024-01-15T10:30+01:00[No").unwrap() + 1);
}
