//! Unit tests of the conformance runner's pieces (spec 006). The runner's module is shared with the
//! test targets that run the collection; these tests need no database.
#![allow(dead_code)]

#[path = "conformance/mod.rs"]
mod conformance;

use conformance::case::{parse_case, Expectation, Kind};
use conformance::settings;

// --- T005: case reading ------------------------------------------------------------------

#[test]
fn a_case_reads_its_requirements_settings_transaction_and_expectations() {
    let text = r#"
/** A relationship without a type. */
{requires: ["vectors"], settings: {tags: "error", maxDiagnostics: 5}, transaction: true}
(:Error {code: "rel.no-type", line: 2, column: 4, offset: 31})
(:Warning {code: "load.anonymous"})
(:Counts {nodesCreated: 2, total: 7})
(:Applied {code: "rel.no-type", count: 3})
(:Remaining {code: "rel.undirected", line: 1, column: 2, offset: 3})
"#;
    let c = parse_case(Kind::Reject, "x", text).unwrap();
    assert_eq!(c.requires, ["vectors"]);
    assert!(c.transaction);
    assert_eq!(c.settings.len(), 2);
    assert_eq!(c.expectations.len(), 5);
    match &c.expectations[0] {
        Expectation::Error(d) => {
            assert_eq!(d.code, "rel.no-type");
            assert_eq!(d.position, Some((2, 4, 31)));
        }
        other => panic!("{other:?}"),
    }
    match &c.expectations[1] {
        Expectation::Warning(d) => assert_eq!(d.position, None),
        other => panic!("{other:?}"),
    }
    assert!(
        matches!(&c.expectations[2], Expectation::Counts(m) if m["nodesCreated"] == 2 && m["total"] == 7)
    );
}

#[test]
fn an_empty_requires_list_is_valid() {
    let c = parse_case(Kind::Load, "x", "{requires: []}\n").unwrap();
    assert!(c.requires.is_empty() && !c.transaction && c.settings.is_empty());
}

#[test]
fn a_case_without_requires_fails() {
    let e = parse_case(Kind::Load, "x", "{settings: {}}\n").unwrap_err();
    assert!(e.contains("requires"), "{e}");
    let e = parse_case(Kind::Load, "x", "(:Error {code: \"a\"})\n").unwrap_err();
    assert!(e.contains("requires"), "{e}");
}

#[test]
fn an_unknown_header_key_fails() {
    let e = parse_case(Kind::Load, "x", "{requires: [], kind: \"load\"}\n").unwrap_err();
    assert!(e.contains("kind"), "{e}");
}

#[test]
fn an_unknown_label_or_counts_key_fails() {
    let e = parse_case(Kind::Load, "x", "{requires: []}\n(:Oops {code: \"a\"})\n").unwrap_err();
    assert!(e.contains("Oops"), "{e}");
    let e = parse_case(
        Kind::Load,
        "x",
        "{requires: []}\n(:Counts {nodesMade: 1})\n",
    )
    .unwrap_err();
    assert!(e.contains("nodesMade"), "{e}");
}

#[test]
fn an_error_node_needs_a_code_and_all_or_none_of_the_position() {
    assert!(parse_case(Kind::Reject, "x", "{requires: []}\n(:Error {line: 1})\n").is_err());
    assert!(parse_case(
        Kind::Reject,
        "x",
        "{requires: []}\n(:Error {code: \"a\", line: 1})\n"
    )
    .is_err());
}

#[test]
fn a_malformed_case_file_is_an_error_naming_the_case() {
    let e = parse_case(Kind::Reject, "my-case", "{requires: [}\n").unwrap_err();
    assert!(e.contains("my-case"), "{e}");
}

#[test]
fn a_case_directory_must_hold_the_files_of_its_kind_and_no_others() {
    use conformance::case::read_case;
    let dir = scratch("files");
    let write = |name: &str, body: &str| std::fs::write(dir.join(name), body).unwrap();
    write("case.gram", "{requires: []}\n");
    // reject needs an input
    assert!(read_case(&dir, Kind::Reject, "x")
        .unwrap_err()
        .contains("input"));
    write("input.txt", "(a)\n");
    assert!(read_case(&dir, Kind::Reject, "x").is_ok());
    // reject does not use expected.gram
    write("expected.gram", "(a)\n");
    assert!(read_case(&dir, Kind::Reject, "x")
        .unwrap_err()
        .contains("expected.gram"));
    // load needs expected.gram and input.txt
    assert!(read_case(&dir, Kind::Load, "x").is_ok());
    std::fs::remove_file(dir.join("expected.gram")).unwrap();
    assert!(read_case(&dir, Kind::Load, "x")
        .unwrap_err()
        .contains("expected.gram"));
    // invalid UTF-8 text goes in input.bin
    std::fs::remove_file(dir.join("input.txt")).unwrap();
    std::fs::write(dir.join("input.bin"), [0xff, 0xfe]).unwrap();
    assert!(read_case(&dir, Kind::Reject, "x").is_ok());
}

fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("conformance-unit-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

// --- T005: settings ----------------------------------------------------------------------

#[test]
fn load_settings_map_from_lower_camel_case_and_default_when_absent() {
    use relate_gram::settings::TagPolicy;
    let c = parse_case(
        Kind::Load,
        "x",
        "{requires: [], settings: {tags: \"error\", maxDiagnostics: 5, nodeIdProperty: \"id\", fileName: \"a.gram\"}}\n",
    )
    .unwrap();
    let s = settings::load_settings(&c.settings).unwrap();
    assert_eq!(s.tags, TagPolicy::Error);
    assert_eq!(s.max_diagnostics, 5);
    assert_eq!(s.node_id_property.as_deref(), Some("id"));
    assert_eq!(s.file_name.as_deref(), Some("a.gram"));
    assert_eq!(s.relationship_id_property, None);
    let d = settings::load_settings(&Default::default()).unwrap();
    assert_eq!(d, relate_gram::settings::LoadSettings::default());
}

#[test]
fn export_and_fix_settings_map_too() {
    use relate_gram::fix::{Direction, OctalAs};
    use relate_gram::settings::IdentityMode;
    let c = parse_case(
        Kind::Export,
        "x",
        "{requires: [], settings: {identity: \"property\", nodeIdProperty: \"k\", relType: \"KNOWS\", direction: \"left\", octal: \"decimal\", annotate: true}}\n",
    )
    .unwrap();
    let e = settings::export_settings(&c.settings).unwrap_err();
    assert!(
        e.contains("is not a setting of an export"),
        "an export does not take a fix option: {e}"
    );
    let c2 = parse_case(
        Kind::Export,
        "x",
        "{requires: [], settings: {identity: \"property\", nodeIdProperty: \"k\"}}\n",
    )
    .unwrap();
    let e = settings::export_settings(&c2.settings).unwrap();
    assert_eq!(e.identity, IdentityMode::Property);
    assert_eq!(e.node_id_property.as_deref(), Some("k"));
    let c3 = parse_case(Kind::Fix, "x", "{requires: [], settings: {relType: \"KNOWS\", direction: \"left\", octal: \"decimal\", annotate: true}}\n").unwrap();
    let f = settings::fix_options(&c3.settings).unwrap();
    assert_eq!(f.rel_type.as_deref(), Some("KNOWS"));
    assert_eq!(f.direction, Some(Direction::Left));
    assert_eq!(f.octal, Some(OctalAs::Decimal));
    assert!(f.annotate);
}

#[test]
fn an_unknown_setting_or_a_wrong_type_fails() {
    let c = parse_case(
        Kind::Load,
        "x",
        "{requires: [], settings: {tagz: \"error\"}}\n",
    )
    .unwrap();
    assert!(settings::load_settings(&c.settings)
        .unwrap_err()
        .contains("tagz"));
    let c = parse_case(
        Kind::Load,
        "x",
        "{requires: [], settings: {maxDiagnostics: \"five\"}}\n",
    )
    .unwrap();
    assert!(settings::load_settings(&c.settings)
        .unwrap_err()
        .contains("maxDiagnostics"));
    let c = parse_case(
        Kind::Load,
        "x",
        "{requires: [], settings: {tags: \"loud\"}}\n",
    )
    .unwrap();
    assert!(settings::load_settings(&c.settings)
        .unwrap_err()
        .contains("tags"));
}

// --- T008: the value table and graph lines -------------------------------------------------

mod value_table {
    use super::conformance::{graph, value};
    use gram_codec::parse_gram_with_header;

    fn db() -> (relate::Database, relate::Connection) {
        let db = relate::Database::open(":memory:").unwrap();
        let c = db.connect().unwrap();
        (db, c)
    }

    /// The canonical text of the one property `v` of the gram text `(a {v: <gram>})`.
    fn expected_text(gram: &str) -> String {
        let (_, patterns) = parse_gram_with_header(&format!("(a {{v: {gram}}})")).unwrap();
        let v = &patterns[0].value.properties["v"];
        value::expected_value(v).unwrap().to_text().unwrap()
    }

    /// The canonical text of the one property `v` of a node created with the Cypher expression.
    fn database_text(cypher: &str) -> String {
        let (_db, mut c) = db();
        c.query(&format!("CREATE (:T {{v: {cypher}}})")).unwrap();
        let g = graph::dump_database(&mut c, false).unwrap();
        assert_eq!(g.nodes.len(), 1);
        let line = &g.nodes[0];
        line.strip_prefix(":T {v: ")
            .and_then(|l| l.strip_suffix('}'))
            .unwrap()
            .to_string()
    }

    fn same(gram: &str, cypher: &str) {
        assert_eq!(
            expected_text(gram),
            database_text(cypher),
            "{gram} against {cypher}"
        );
    }

    #[test]
    fn scalars_meet() {
        same("5", "5");
        same("-0", "0");
        same("1.5", "1.5");
        same("1.0", "1.0");
        same("-0.0", "-0.0");
        same("true", "true");
        same("\"a b\"", "'a b'");
        same("active", "'active'");
        same("\"\"", "''");
        same("\"a\\nb\\\"q\\\"\"", "'a\\nb\"q\"'");
    }

    #[test]
    fn lists_meet() {
        same("[1, 2]", "[1, 2]");
        same("[1.0, 2.5]", "[1.0, 2.5]");
        same("[\"a\", \"b, c\"]", "['a', 'b, c']");
        same("[true, false]", "[true, false]");
        same("[]", "[]");
    }

    #[test]
    fn typed_values_meet_at_their_normal_content() {
        same("date`2024-01-15`", "date('2024-01-15')");
        same("time`10:30Z`", "time('10:30:00Z')");
        same("time`10:30:00+01:00`", "time('10:30:00+01:00')");
        same("localtime`10:30:00.50`", "localtime('10:30:00.5')");
        same(
            "datetime`2024-01-15T10:30:00+01:00[Europe/Paris]`",
            "datetime('2024-01-15T10:30:00+01:00[Europe/Paris]')",
        );
        same(
            "localdatetime`2024-01-15T10:30`",
            "localdatetime('2024-01-15T10:30:00')",
        );
        same(
            "duration`P1Y2M3DT4H5M6.789S`",
            "duration('P1Y2M3DT4H5M6.789S')",
        );
        same(
            "point`SRID=4326;1.5 2`",
            "point({x: 1.5, y: 2.0, srid: 4326})",
        );
        same(
            "point`SRID=9157;1 2 3`",
            "point({x: 1.0, y: 2.0, z: 3.0, srid: 9157})",
        );
        same(
            "uuid`123E4567-E89B-12D3-A456-426614174000`",
            "uuid('123e4567-e89b-12d3-a456-426614174000')",
        );
        same(
            "[date`2024-01-15`, date`2024-02-01`]",
            "[date('2024-01-15'), date('2024-02-01')]",
        );
    }

    #[test]
    fn float_vectors_meet_as_embeddings() {
        use gram_value::tags::{Embedding, EmbeddingDtype};
        let content = |dtype, values: &[f64]| {
            let bytes = relate_gram::__test_support::narrow(dtype, values);
            Embedding::new(dtype, values.len(), bytes)
                .unwrap()
                .to_normal_content()
        };
        let f32c = content(EmbeddingDtype::F32, &[1.0, 2.5]);
        same(
            &format!("embedding`{f32c}`"),
            "vector([1.0, 2.5], 2, FLOAT32)",
        );
        let f64c = content(EmbeddingDtype::F64, &[0.1]);
        same(&format!("embedding`{f64c}`"), "vector([0.1], 1, FLOAT64)");
    }

    #[test]
    fn values_that_differ_do_not_meet() {
        assert_ne!(expected_text("1"), expected_text("1.0"));
        assert_ne!(expected_text("1"), expected_text("\"1\""));
        assert_ne!(
            expected_text("date`2024-01-15`"),
            expected_text("\"2024-01-15\"")
        );
        assert_ne!(expected_text("[1, 2]"), expected_text("[2, 1]"));
        assert_ne!(
            expected_text("point`SRID=4326;1 2`"),
            expected_text("point`SRID=7203;1 2`")
        );
    }

    #[test]
    fn an_integer_vector_has_no_gram_form_and_is_reported() {
        let (_db, mut c) = db();
        c.query("CREATE (:T {v: vector([1, 2], 2, INTEGER8)})")
            .unwrap();
        let e = graph::dump_database(&mut c, false).unwrap_err();
        assert!(e.contains("INTEGER8"), "{e}");
    }

    #[test]
    fn a_map_range_or_measurement_in_an_expected_graph_is_an_error() {
        for gram in ["{k: 1}", "1..5", "5kg"] {
            let (_, p) = parse_gram_with_header(&format!("(a {{v: {gram}}})")).unwrap();
            assert!(
                value::expected_value(&p[0].value.properties["v"]).is_err(),
                "{gram}"
            );
        }
    }
}

mod graph_lines {
    use super::conformance::graph;

    fn db() -> (relate::Database, relate::Connection) {
        let db = relate::Database::open(":memory:").unwrap();
        let c = db.connect().unwrap();
        (db, c)
    }

    #[test]
    fn a_node_line_has_sorted_labels_and_keys_and_a_relationship_line_holds_its_ends() {
        let (_db, mut c) = db();
        c.query("CREATE (:B:A {z: 1, a: 'x'})-[:R {w: 2.5}]->(:C)")
            .unwrap();
        let g = graph::dump_database(&mut c, false).unwrap();
        assert_eq!(g.nodes, [":A:B {a: x, z: 1}", ":C {}"]);
        assert_eq!(
            g.relationships,
            ["(:A:B {a: x, z: 1}) -[R {w: 2.5}]-> (:C {})"]
        );
    }

    #[test]
    fn graphs_are_equal_by_content_and_not_by_order_or_internal_id() {
        let (_d1, mut a) = db();
        let (_d2, mut b) = db();
        a.query("CREATE (:N {k: 1}), (:N {k: 2})").unwrap();
        b.query("CREATE (:N {k: 2}), (:N {k: 1})").unwrap();
        assert_eq!(
            graph::dump_database(&mut a, false).unwrap(),
            graph::dump_database(&mut b, false).unwrap()
        );
    }

    #[test]
    fn graphs_that_differ_in_a_label_a_key_or_a_value_are_not_equal() {
        let one = |q: &str| {
            let (_d, mut c) = db();
            c.query(q).unwrap();
            graph::dump_database(&mut c, false).unwrap()
        };
        let base = one("CREATE (:N {k: 1})");
        assert_ne!(base, one("CREATE (:M {k: 1})"));
        assert_ne!(base, one("CREATE (:N {j: 1})"));
        assert_ne!(base, one("CREATE (:N {k: 2})"));
        assert_ne!(base, one("CREATE (:N {k: 1.0})"));
    }

    #[test]
    fn a_generated_identifier_is_dropped_only_when_asked() {
        let (_db, mut c) = db();
        c.query("CREATE (:N {`@id`: 'n0', k: 1}), (:N {`@id`: 'alice', k: 2})")
            .unwrap();
        let kept = graph::dump_database(&mut c, false).unwrap();
        assert!(kept.nodes.iter().any(|l| l.contains("@id: n0")));
        let stripped = graph::dump_database(&mut c, true).unwrap();
        assert!(
            stripped.nodes.iter().all(|l| !l.contains("@id: n0")),
            "{:?}",
            stripped.nodes
        );
        assert!(
            stripped.nodes.iter().any(|l| l.contains("@id: alice")),
            "{:?}",
            stripped.nodes
        );
    }
}
