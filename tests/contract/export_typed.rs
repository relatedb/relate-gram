//! Typed values are written as tagged strings and round-trip (ADR-011 sections 2.2 and 2.6),
//! specs/002-typed-tags User Story 3.
use super::common::*;

/// A database with every typed value on a node, and some on a relationship, built with
/// Cypher constructors as Relate documents them.
fn build(c: &mut relate::Connection) {
    run(
        c,
        "CREATE (a:Thing {d: date('2024-01-15'), t: time('10:30:00.123456789+05:30'), \
         lt: localtime('10:30:00'), dt: datetime('2024-01-15T10:30:00+01:00[Europe/Paris]'), \
         ldt: localdatetime('2024-01-15T10:30:00'), du: duration('P1DT2H'), \
         neg: duration('-P1DT2H'), mixed: duration('P1DT-2H'), \
         p: point({latitude: 40.7, longitude: -74.0}), c: point({x: 1.0, y: 2.0, z: 3.0}), \
         u: uuid('123E4567-E89B-12D3-A456-426614174000')}), (b:Thing) \
         CREATE (a)-[:LINK {since: date('2020-05-05'), span: duration('PT1S'), \
         at: point({x: 1.5, y: 2.5})}]->(b)",
    );
}

#[test]
fn every_typed_value_is_written_as_a_tagged_string() {
    let (_db, mut c) = fresh_db();
    build(&mut c);
    let text = export_str(&mut c).unwrap();
    for want in [
        "d: date`2024-01-15`",
        "t: time`10:30:00.123456789+05:30`",
        "lt: localtime`10:30:00`",
        "dt: datetime`2024-01-15T10:30:00+01:00[Europe/Paris]`",
        "ldt: localdatetime`2024-01-15T10:30:00`",
        "du: duration`P1DT2H`",
        "neg: duration`P-1DT-2H`",
        "mixed: duration`P1DT-2H`",
        "u: uuid`123e4567-e89b-12d3-a456-426614174000`",
        "since: date`2020-05-05`",
        "span: duration`PT1S`",
    ] {
        assert!(text.contains(want), "missing {want} in:\n{text}");
    }
}

#[test]
fn points_are_written_with_their_srid_and_longitude_first() {
    let (_db, mut c) = fresh_db();
    build(&mut c);
    let text = export_str(&mut c).unwrap();
    assert!(
        text.contains("p: point`SRID=4326;POINT(-74.0 40.7)`"),
        "{text}"
    );
    assert!(
        text.contains("c: point`SRID=9157;POINT Z(1.0 2.0 3.0)`"),
        "{text}"
    );
    assert!(
        text.contains("at: point`SRID=7203;POINT(1.5 2.5)`"),
        "{text}"
    );
}

#[test]
fn an_export_of_unchanged_data_is_byte_identical() {
    let (_db, mut c) = fresh_db();
    build(&mut c);
    assert_eq!(export_str(&mut c).unwrap(), export_str(&mut c).unwrap());
}

#[test]
fn export_then_load_gives_equal_types_and_values() {
    let (_db, mut c) = fresh_db();
    build(&mut c);
    let text = export_str(&mut c).unwrap();
    let (_db2, mut d) = fresh_db();
    load_str(&mut d, &text).unwrap();
    assert_eq!(
        dump_graph_without_ids(&mut c),
        dump_graph_without_ids(&mut d)
    );
    // and the typed properties have the types the original had
    let q = "MATCH (n:Thing) WHERE n.d IS NOT NULL \
             RETURN valueType(n.d), valueType(n.t), valueType(n.lt), valueType(n.dt), \
             valueType(n.ldt), valueType(n.du), valueType(n.p), valueType(n.u)";
    assert_eq!(run(&mut c, q), run(&mut d, q));
}

#[test]
fn a_list_of_one_typed_kind_is_written_as_an_array_of_tagged_strings() {
    let (_db, mut c) = fresh_db();
    run(
        &mut c,
        "CREATE (:L {dates: [date('2024-01-01'), date('2024-02-02')], \
         spans: [duration('P1D'), duration('PT1H')], \
         places: [point({x: 1.0, y: 2.0}), point({x: 1.0, y: 2.0, z: 3.0})], \
         ids: [uuid('123e4567-e89b-12d3-a456-426614174000')]})",
    );
    let text = export_str(&mut c).unwrap();
    for want in [
        "dates: [date`2024-01-01`, date`2024-02-02`]",
        "spans: [duration`P1D`, duration`PT1H`]",
        "places: [point`SRID=7203;POINT(1.0 2.0)`, point`SRID=9157;POINT Z(1.0 2.0 3.0)`]",
        "ids: [uuid`123e4567-e89b-12d3-a456-426614174000`]",
    ] {
        assert!(text.contains(want), "missing {want} in:\n{text}");
    }
    let (_db2, mut d) = fresh_db();
    load_str(&mut d, &text).unwrap();
    assert_eq!(
        dump_graph_without_ids(&mut c),
        dump_graph_without_ids(&mut d)
    );
}

#[test]
fn a_list_that_mixes_temporal_kinds_fails_the_export_and_is_listed() {
    let (_db, mut c) = fresh_db();
    run(
        &mut c,
        "CREATE (:M {xs: [date('2024-01-01'), datetime('2024-01-01T00:00:00Z')]})",
    );
    let f = export_str(&mut c).unwrap_err();
    assert_eq!(codes_of(&f), vec!["export.value"]);
    assert!(
        f.diagnostics[0].message.contains("xs"),
        "{}",
        f.diagnostics[0].message
    );
}
