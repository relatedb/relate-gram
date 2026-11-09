//! Story 2, scenarios 3 to 6 (ADR-001, ADR-003, ADR-006, ADR-007).
use super::common::*;
use relate_gram::diagnostics::code;
use relate_gram::settings::{LoadSettings, TagPolicy};

fn errors(f: &relate_gram::LoadFailure) -> Vec<&'static str> {
    f.diagnostics
        .iter()
        .filter(|d| d.is_error())
        .map(|d| d.code)
        .collect()
}

#[test]
fn several_errors_are_reported_together_in_source_order() {
    let (_db, mut c) = fresh_db();
    let f = load_str(&mut c, "(a:N)-->(b:N)\n(c:N)--(d:N)\n(e:N {r: 1..2})").unwrap_err();
    assert_eq!(
        errors(&f),
        vec![code::REL_NO_TYPE, code::REL_UNDIRECTED, code::VALUE_RANGE]
    );
    assert_eq!(f.total, 3);
    let lines: Vec<usize> = f.diagnostics.iter().map(|d| d.span.unwrap().line).collect();
    assert_eq!(lines, vec![1, 2, 3]);
}

#[test]
fn the_cap_limits_the_list_and_total_counts_all() {
    let (_db, mut c) = fresh_db();
    let settings = LoadSettings {
        max_diagnostics: 2,
        ..LoadSettings::default()
    };
    let f = relate_gram::load(
        &mut c,
        b"(a:N)-->(b:N)\n(c:N)--(d:N)\n(e:N {r: 1..2})",
        &settings,
    )
    .unwrap_err();
    assert_eq!(f.diagnostics.len(), 2);
    assert_eq!(f.total, 3);
}

#[test]
fn nothing_before_an_error_is_written() {
    let (_db, mut c) = fresh_db();
    load_str(&mut c, "(a:Good)\n(b:Good)\n(c)-->(d)").unwrap_err();
    assert_eq!(
        run(&mut c, "MATCH (n) RETURN count(n)")[0][0],
        relate::Value::Int(0)
    );
}

#[test]
fn the_tag_error_policy_reports_every_tagged_string_and_loads_nothing() {
    let (_db, mut c) = fresh_db();
    let settings = LoadSettings {
        tags: TagPolicy::Error,
        ..LoadSettings::default()
    };
    let f = relate_gram::load(&mut c, b"(a {s: md`x`})\n(b {t: ```py\ny\n```})", &settings)
        .unwrap_err();
    assert_eq!(errors(&f), vec![code::VALUE_TAG, code::VALUE_TAG]);
    assert_eq!(f.diagnostics[0].span.unwrap().line, 1);
    assert_eq!(f.diagnostics[1].span.unwrap().line, 2);
    assert_eq!(
        run(&mut c, "MATCH (n) RETURN count(n)")[0][0],
        relate::Value::Int(0)
    );
}

#[test]
fn the_default_tag_policy_warns_and_loads() {
    let (_db, mut c) = fresh_db();
    let r = load_str(&mut c, "(a {s: md`x`})").unwrap();
    assert_eq!(r.counts.tags_dropped, 1);
}

#[test]
fn the_record_is_checked_and_its_properties_reported_as_not_stored() {
    let (_db, mut c) = fresh_db();
    let r = load_str(
        &mut c,
        "{kind: \"property-graph\", name: \"roster\", revision: 4}\n(a)",
    )
    .unwrap();
    let w = r
        .diagnostics
        .iter()
        .find(|d| d.code == code::FILE_RECORD_NOT_STORED)
        .expect("a warning");
    assert!(
        w.message.contains("name") && w.message.contains("revision"),
        "{}",
        w.message
    );
    assert!(
        !w.message.contains("kind"),
        "kind is checked, not listed: {}",
        w.message
    );
    // the record changes nothing in the graph
    assert_eq!(
        run(&mut c, "MATCH (n) RETURN count(n)")[0][0],
        relate::Value::Int(1)
    );
}

#[test]
fn a_record_with_only_kind_gives_no_warning() {
    let (_db, mut c) = fresh_db();
    let r = load_str(&mut c, "{kind: \"property-graph\"}\n(a)").unwrap();
    assert!(!r
        .diagnostics
        .iter()
        .any(|d| d.code == code::FILE_RECORD_NOT_STORED));
}

#[test]
fn record_values_are_checked_under_the_value_mapping() {
    let (_db, mut c) = fresh_db();
    let f = load_str(&mut c, "{name: 1..2}\n(a)").unwrap_err();
    assert_eq!(errors(&f), vec![code::VALUE_RANGE]);
    let f = load_str(&mut c, "{\"@x\": 1}\n(a)").unwrap_err();
    assert_eq!(errors(&f), vec![code::KEY_RESERVED]);
}

#[test]
fn a_kind_other_than_property_graph_fails_with_the_assumption_stated() {
    let (_db, mut c) = fresh_db();
    let f = load_str(&mut c, "{kind: \"schema\"}\n(a)").unwrap_err();
    assert_eq!(errors(&f), vec![code::FILE_KIND]);
    assert!(f.diagnostics[0]
        .message
        .contains("relate-gram loads only kind 'property-graph'"));
    assert!(f.diagnostics[0].message.contains("schema"));
}

#[test]
fn the_file_name_kind_is_checked() {
    let (_db, mut c) = fresh_db();
    let with = |name: &str| LoadSettings {
        file_name: Some(name.to_string()),
        ..LoadSettings::default()
    };
    let f = relate_gram::load(&mut c, b"(a)", &with("roster.schema.gram")).unwrap_err();
    assert_eq!(errors(&f), vec![code::FILE_KIND]);
    assert_eq!(f.diagnostics[0].span, None);
    assert!(f.diagnostics[0].message.contains("schema"));
    // only the last segment before `.gram` is the kind
    assert!(relate_gram::load(&mut c, b"(a)", &with("roster.v2.gram")).is_err());
    assert!(relate_gram::load(&mut c, b"(a)", &with("roster.property-graph.gram")).is_ok());
    assert!(relate_gram::load(&mut c, b"(b)", &with("roster.gram")).is_ok());
    assert!(relate_gram::load(&mut c, b"(c)", &with("/some/dir/people.gram")).is_ok());
}

#[test]
fn a_connection_already_in_a_transaction_is_refused() {
    let (_db, mut c) = fresh_db();
    c.begin().unwrap();
    let f = load_str(&mut c, "(a)").unwrap_err();
    assert_eq!(errors(&f), vec![code::LOAD_IN_TRANSACTION]);
    assert!(c.in_transaction(), "the caller's transaction is left alone");
    c.rollback().unwrap();
}

#[test]
fn a_relate_error_is_reported_and_rolled_back() {
    let (_db, mut c) = fresh_db();
    run(
        &mut c,
        "CREATE CONSTRAINT FOR (n:L) REQUIRE n.`@id` IS UNIQUE",
    );
    run(&mut c, "CREATE (:L {`@id`: 'a'})");
    let f = load_str(&mut c, "(fresh:Other)\n(a:L)").unwrap_err();
    assert_eq!(errors(&f), vec![code::LOAD_DATABASE_ERROR]);
    assert_eq!(f.diagnostics[0].span, None);
    assert!(!c.in_transaction());
    assert_eq!(
        run(&mut c, "MATCH (n) RETURN count(n)")[0][0],
        relate::Value::Int(1)
    );
}

#[test]
fn repeated_and_conflicting_constructs_point_back_at_the_first() {
    let (_db, mut c) = fresh_db();
    let f = load_str(&mut c, "(a {k: 1, k: 2})").unwrap_err();
    assert_eq!(f.diagnostics[0].code, code::KEY_DUPLICATE);
    assert_eq!(f.diagnostics[0].span.unwrap().column, 13);
    assert_eq!(f.diagnostics[0].related[0].1.column, 7);

    let f = load_str(&mut c, "(a:L)\n(a:M)").unwrap_err();
    assert_eq!(f.diagnostics[0].code, code::ID_DUPLICATE_DEFINITION);
    assert_eq!(
        (
            f.diagnostics[0].span.unwrap().line,
            f.diagnostics[0].related[0].1.line
        ),
        (2, 1)
    );

    let f = load_str(&mut c, "(a)-[a:R]->(b)").unwrap_err();
    assert_eq!(f.diagnostics[0].code, code::ID_KIND_CONFLICT);
    assert_eq!(f.diagnostics[0].related[0].1.column, 2);
}
