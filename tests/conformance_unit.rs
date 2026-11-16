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

// --- T010: the expected-graph rules ----------------------------------------------------------

mod expected_rules {
    use super::conformance::expected::{read_expected, Identity};

    fn read(text: &str) -> Result<super::conformance::graph::Graph, String> {
        read_expected(text, &Identity::default())
    }

    #[test]
    fn nodes_and_single_hop_relationships_read_as_lines_with_the_identifier_as_the_id() {
        let g = read("(ada:Person {name: \"Ada\", born: 1815})\n(bob:Person)\n(ada)-[:KNOWS {since: 1.5}]->(bob)\n(:Doc {t: \"x\"})\n").unwrap();
        assert_eq!(
            g.nodes,
            [
                ":Doc {t: x}",
                ":Person {@id: ada, born: 1815, name: Ada}",
                ":Person {@id: bob}"
            ]
        );
        assert_eq!(
            g.relationships,
            ["(:Person {@id: ada, born: 1815, name: Ada}) -[KNOWS {since: 1.5}]-> (:Person {@id: bob})"]
        );
    }

    #[test]
    fn an_identity_property_makes_identifiers_handles_with_no_id_expected() {
        let id = Identity {
            node_property: Some("key".into()),
            relationship_property: None,
        };
        let g = read_expected(
            "(a:N {key: \"a\"})\n(b:N {key: \"b\"})\n(a)-[:R]->(b)\n",
            &id,
        )
        .unwrap();
        assert_eq!(g.nodes, [":N {key: a}", ":N {key: b}"]);
        assert_eq!(g.relationships, ["(:N {key: a}) -[R {}]-> (:N {key: b})"]);
    }

    fn breaks(text: &str, needle: &str) {
        let e = read(text).unwrap_err();
        assert!(
            e.contains(needle),
            "wanted `{needle}` in `{e}` for {text:?}"
        );
    }

    #[test]
    fn each_rule_breach_fails_naming_the_rule() {
        breaks("(a)-[:R]->(b)-[:S]->(c)\n", "path");
        breaks("(a)-[:R]->(b)\n(a)\n(b)\n", "forward reference");
        breaks("@k(1) (a)\n", "annotation");
        breaks("(a {m: {x: 1}})\n", "map");
        breaks("(a)\n(a:N)\n", "defined more than once");
        breaks("(a)\n(a)-[:R]->(b)\n", "not defined");
        breaks("(a)\n(a:N)-[:R]->(b)\n", "identifier alone");
        breaks("(a)\n(b)\n(c)\n[s | a, b, c]\n", "at most two");
        breaks("{kind: \"x\"}\n(a)\n", "header");
        breaks("(a)\n(b)\n(:N)-[:R]->(b)\n", "identifier alone");
    }

    #[test]
    fn a_relationship_with_its_own_identifier_has_an_id() {
        let g = read("(a)\n(b)\n(a)-[r:R]->(b)\n").unwrap();
        assert_eq!(g.relationships, ["({@id: a}) -[R {@id: r}]-> ({@id: b})"]);
    }
}

// --- T013: diagnostics and counts ------------------------------------------------------------

mod diagnostics_matching {
    use super::conformance::case::Diag;
    use super::conformance::expect::{compare_counts, compare_diagnostics, split};
    use relate_gram::diagnostics::{Diagnostic, Span};
    use relate_gram::LoadCounts;
    use std::collections::BTreeMap;

    fn span(line: usize, column: usize, start: usize) -> Span {
        Span {
            start,
            end: start + 1,
            line,
            column,
        }
    }

    fn d(code: &str, p: Option<(usize, usize, usize)>) -> Diag {
        Diag {
            code: code.into(),
            position: p,
        }
    }

    fn lib_error(code: &'static str, line: usize, column: usize, offset: usize) -> Diagnostic {
        Diagnostic::error(code, span(line, column, offset), "m")
    }

    #[test]
    fn matching_diagnostics_pass_in_any_order() {
        let got = [lib_error("b.x", 3, 1, 10), lib_error("a.y", 2, 4, 5)];
        let (errors, warnings) = split(&got);
        assert!(warnings.is_empty());
        compare_diagnostics(
            "Error",
            &[d("a.y", Some((2, 4, 5))), d("b.x", Some((3, 1, 10)))],
            &errors,
        )
        .unwrap();
    }

    #[test]
    fn a_different_code_line_column_or_offset_fails_naming_the_part() {
        let (errors, _) = split(&[lib_error("a.y", 2, 4, 5)]);
        let e = compare_diagnostics("Error", &[d("a.z", Some((2, 4, 5)))], &errors).unwrap_err();
        assert!(e.contains("a.z") && e.contains("a.y"), "{e}");
        for (want, part) in [
            ((3, 4, 5), "line"),
            ((2, 9, 5), "column"),
            ((2, 4, 9), "offset"),
        ] {
            let e = compare_diagnostics("Error", &[d("a.y", Some(want))], &errors).unwrap_err();
            assert!(e.contains(part), "{part}: {e}");
        }
    }

    #[test]
    fn an_extra_or_a_missing_diagnostic_fails() {
        let (errors, _) = split(&[lib_error("a.y", 2, 4, 5), lib_error("b.x", 3, 1, 10)]);
        let e = compare_diagnostics("Error", &[d("a.y", Some((2, 4, 5)))], &errors).unwrap_err();
        assert!(e.contains("unexpected") && e.contains("b.x"), "{e}");
        let e = compare_diagnostics(
            "Error",
            &[d("a.y", Some((2, 4, 5))), d("c.z", Some((1, 1, 0)))],
            &split(&[lib_error("a.y", 2, 4, 5)]).0,
        )
        .unwrap_err();
        assert!(e.contains("missing") && e.contains("c.z"), "{e}");
    }

    #[test]
    fn a_diagnostic_with_no_position_matches_by_code_alone() {
        let got = [Diagnostic::warning_without_span("load.anonymous", "m")];
        let (_, warnings) = split(&got);
        compare_diagnostics("Warning", &[d("load.anonymous", None)], &warnings).unwrap();
        assert!(compare_diagnostics(
            "Warning",
            &[d("load.anonymous", Some((1, 1, 0)))],
            &warnings
        )
        .is_err());
    }

    #[test]
    fn errors_and_warnings_are_split_by_severity() {
        let got = [
            lib_error("a.y", 1, 1, 0),
            Diagnostic::warning("w.z", span(2, 1, 7), "m"),
        ];
        let (errors, warnings) = split(&got);
        assert_eq!((errors.len(), warnings.len()), (1, 1));
    }

    #[test]
    fn counts_compare_exactly_the_keys_listed() {
        let got = LoadCounts {
            nodes_created: 2,
            nodes_updated: 1,
            relationships_created: 3,
            relationships_updated: 0,
            elements_without_identifier: 4,
            reference_only_nodes: 0,
            tags_dropped: 5,
        };
        let want = |pairs: &[(&str, i64)]| {
            pairs
                .iter()
                .map(|(k, v)| (k.to_string(), *v))
                .collect::<BTreeMap<_, _>>()
        };
        compare_counts(
            &want(&[("nodesCreated", 2), ("tagsDropped", 5)]),
            &got,
            None,
        )
        .unwrap();
        // keys that are not listed are not checked
        compare_counts(&want(&[("nodesUpdated", 1)]), &got, None).unwrap();
        let e = compare_counts(&want(&[("nodesCreated", 9)]), &got, None).unwrap_err();
        assert!(
            e.contains("nodesCreated") && e.contains('9') && e.contains('2'),
            "{e}"
        );
        compare_counts(&want(&[("total", 7)]), &got, Some(7)).unwrap();
        assert!(compare_counts(&want(&[("total", 7)]), &got, Some(8)).is_err());
        assert!(
            compare_counts(&want(&[("total", 7)]), &got, None).is_err(),
            "no total available"
        );
    }
}

// --- T016: the reject runner ----------------------------------------------------------------

mod reject_runner {
    use super::conformance::case::{parse_case, Kind};
    use super::conformance::reject::run_reject_text;

    const START: &str = "CREATE (:Existing {k: 1})-[:R]->(:Existing {k: 2});\n";
    const TEXT: &[u8] = b"// a bare arrow has no type\n(a)-->(b)\n";

    fn case(expectations: &str) -> super::conformance::case::Case {
        parse_case(
            Kind::Reject,
            "t",
            &format!("{{requires: []}}\n{expectations}"),
        )
        .unwrap()
    }

    const GOOD: &str = "(:Error {code: \"rel.no-type\", line: 2, column: 4, offset: 31})\n";

    #[test]
    fn a_correct_case_passes() {
        run_reject_text(&case(GOOD), TEXT, START).unwrap();
    }

    #[test]
    fn a_wrong_code_line_column_or_offset_fails_naming_the_part() {
        for (bad, part) in [
            (
                "(:Error {code: \"rel.undirected\", line: 2, column: 4, offset: 31})\n",
                "rel.undirected",
            ),
            (
                "(:Error {code: \"rel.no-type\", line: 3, column: 4, offset: 31})\n",
                "line",
            ),
            (
                "(:Error {code: \"rel.no-type\", line: 2, column: 5, offset: 31})\n",
                "column",
            ),
            (
                "(:Error {code: \"rel.no-type\", line: 2, column: 4, offset: 32})\n",
                "offset",
            ),
        ] {
            let e = run_reject_text(&case(bad), TEXT, START).unwrap_err();
            assert!(e.contains(part), "{part}: {e}");
        }
    }

    #[test]
    fn an_extra_or_a_missing_error_fails() {
        let e = run_reject_text(&case(""), TEXT, START).unwrap_err();
        assert!(e.contains("unexpected") && e.contains("rel.no-type"), "{e}");
        let two =
            format!("{GOOD}(:Error {{code: \"rel.undirected\", line: 1, column: 1, offset: 0}})\n");
        let e = run_reject_text(&case(&two), TEXT, START).unwrap_err();
        assert!(e.contains("missing") && e.contains("rel.undirected"), "{e}");
    }

    #[test]
    fn a_text_that_loads_is_not_a_reject_case() {
        let e = run_reject_text(&case(GOOD), b"(a)-[:T]->(b)\n", START).unwrap_err();
        assert!(
            e.contains("loaded") || e.contains("should be rejected"),
            "{e}"
        );
    }

    #[test]
    fn the_total_is_compared_with_the_errors_found_before_the_cap() {
        let with_total = |n: i64| {
            parse_case(
                Kind::Reject,
                "t",
                &format!("{{requires: [], settings: {{maxDiagnostics: 1}}}}\n(:Error {{code: \"rel.no-type\", line: 2, column: 4, offset: 8}})\n(:Counts {{total: {n}}})\n"),
            )
            .unwrap()
        };
        let text = b"// c\n(a)-->(b)\n(c)-->(d)\n";
        run_reject_text(&with_total(2), text, START).unwrap();
        let e = run_reject_text(&with_total(3), text, START).unwrap_err();
        assert!(e.contains("total"), "{e}");
    }

    #[test]
    fn a_connection_in_a_transaction_is_refused_and_leaves_the_starting_graph() {
        let c = parse_case(Kind::Reject, "t", "{requires: [\"explicit-transactions\"], transaction: true}\n(:Error {code: \"load.in-transaction\"})\n").unwrap();
        run_reject_text(&c, b"(a)\n", START).unwrap();
    }
}

// --- T021: the load runner -------------------------------------------------------------------

mod load_runner {
    use super::conformance::case::{parse_case, Kind};
    use super::conformance::load::run_load_text;

    const TEXT: &[u8] = b"(a:Person {name: \"Ada\"})-[r:KNOWS]->(b:Person)\n";
    const EXPECTED: &str = "(a:Person {name: \"Ada\"})\n(b:Person)\n(a)-[r:KNOWS]->(b)\n";

    fn case(header_and_nodes: &str) -> super::conformance::case::Case {
        parse_case(Kind::Load, "t", header_and_nodes).unwrap()
    }

    fn plain() -> super::conformance::case::Case {
        case("{requires: []}\n(:Counts {nodesCreated: 2, relationshipsCreated: 1})\n")
    }

    #[test]
    fn a_correct_case_passes() {
        run_load_text(&plain(), TEXT, "", EXPECTED).unwrap();
    }

    #[test]
    fn a_graph_that_differs_fails_naming_the_difference() {
        for (expected, needle) in [
            ("(a:Person)\n(b:Person)\n(a)-[r:KNOWS]->(b)\n", "name"),
            (
                "(a:Person {name: \"Bob\"})\n(b:Person)\n(a)-[r:KNOWS]->(b)\n",
                "Bob",
            ),
            (
                "(a:Person {name: \"Ada\"})\n(b:Person)\n(c:Person)\n(a)-[r:KNOWS]->(b)\n",
                "node",
            ),
            ("(a:Person {name: \"Ada\"})\n(b:Person)\n", "relationship"),
        ] {
            let e = run_load_text(&plain(), TEXT, "", expected).unwrap_err();
            assert!(e.contains(needle), "{needle}: {e}");
        }
    }

    #[test]
    fn counts_and_warnings_are_compared() {
        let wrong = case("{requires: []}\n(:Counts {nodesCreated: 3})\n");
        assert!(run_load_text(&wrong, TEXT, "", EXPECTED)
            .unwrap_err()
            .contains("nodesCreated"));
        // an unlisted warning fails; a listed one passes
        let tagged: &[u8] = b"(a {m: md`x`})\n";
        let none = case("{requires: []}\n");
        assert!(run_load_text(&none, tagged, "", "(a {m: \"x\"})\n")
            .unwrap_err()
            .contains("Warning"));
        let listed = case("{requires: []}\n(:Warning {code: \"value.tag-dropped\", line: 1, column: 8, offset: 7})\n");
        run_load_text(&listed, tagged, "", "(a {m: \"x\"})\n").unwrap();
    }

    #[test]
    fn a_start_graph_and_an_identity_property_match_and_update() {
        let c = case("{requires: [], settings: {nodeIdProperty: \"key\"}}\n(:Counts {nodesCreated: 0, nodesUpdated: 1})\n");
        run_load_text(
            &c,
            b"(:N {key: \"a\", y: 2})\n",
            "CREATE (:N {key: 'a', x: 1});\n",
            "(:N {key: \"a\", x: 1, y: 2})\n",
        )
        .unwrap();
    }

    #[test]
    fn a_text_that_does_not_load_is_not_a_load_case() {
        let e = run_load_text(&plain(), b"(a)-->(b)\n", "", EXPECTED).unwrap_err();
        assert!(
            e.contains("did not load") && e.contains("rel.no-type"),
            "{e}"
        );
    }

    #[test]
    fn a_malformed_expected_graph_fails_the_case_naming_the_rule() {
        let e = run_load_text(&plain(), TEXT, "", "(a)-[:R]->(b)-[:S]->(c)\n").unwrap_err();
        assert!(e.contains("path"), "{e}");
    }
}

mod export_runner {
    use super::conformance::case::{parse_case, Case, Kind};
    use super::conformance::export::run_export_text;
    use super::conformance::load::run_load_export;

    const START: &str = "CREATE (a:Person {name: 'Ada'})-[:KNOWS]->(b:Person);\n";
    const TEXT: &[u8] = b"(n0:Person {name: Ada})\n(n1:Person)\n\n(n0)-[r0:KNOWS]->(n1)\n";

    fn export(body: &str) -> Case {
        parse_case(Kind::Export, "t", body).unwrap()
    }

    fn plain() -> Case {
        export("{requires: []}\n")
    }

    #[test]
    fn text_that_matches_byte_for_byte_passes() {
        run_export_text(&plain(), START, Some(TEXT)).unwrap();
    }

    #[test]
    fn a_changed_byte_fails() {
        let mut bad = TEXT.to_vec();
        bad[10] = b'X';
        let e = run_export_text(&plain(), START, Some(&bad)).unwrap_err();
        assert!(e.contains("byte"), "{e}");
        // a trailing newline is part of the text
        let e = run_export_text(&plain(), START, Some(&TEXT[..TEXT.len() - 1])).unwrap_err();
        assert!(e.contains("byte"), "{e}");
    }

    #[test]
    fn an_expected_failure_is_compared_by_code_in_any_order() {
        let c = export(
            "{requires: []}\n(:Error {code: \"export.value\"})\n(:Error {code: \"export.value\"})\n",
        );
        let start =
            "CREATE (:A {v: vector([1, 2], 2, INTEGER16)}), (:B {v: vector([3], 1, INTEGER16)});\n";
        run_export_text(&c, start, None).unwrap();
        // a missing error and an unexpected export both fail
        let one = export("{requires: []}\n(:Error {code: \"export.value\"})\n");
        assert!(run_export_text(&one, start, None).is_err());
        assert!(run_export_text(&c, START, None).is_err());
    }

    #[test]
    fn a_transaction_gives_the_refusal_code() {
        let c = export(
            "{requires: [], transaction: true}\n(:Error {code: \"export.in-transaction\"})\n",
        );
        run_export_text(&c, "", None).unwrap();
    }

    #[test]
    fn a_load_case_with_exported_text_checks_it_and_the_reload() {
        let c = parse_case(
            Kind::Load,
            "t",
            "{requires: []}\n(:Counts {nodesCreated: 2, relationshipsCreated: 1})\n",
        )
        .unwrap();
        let input = b"(a:Person {name: \"Ada\"})-[r:KNOWS]->(b:Person)\n";
        let expected = "(a:Person {name: \"Ada\"})\n(b:Person)\n(a)-[r:KNOWS]->(b)\n";
        // the exporter writes the stored identifiers
        let text = b"(a:Person {`@id`: a})\n";
        let _ = text;
        let exported = b"(a:Person {name: Ada})\n(b:Person)\n\n(a)-[r:KNOWS]->(b)\n";
        run_load_export(&c, input, "", expected, Some(exported)).unwrap();
        let e = run_load_export(&c, input, "", expected, Some(b"(a)\n")).unwrap_err();
        assert!(e.contains("byte"), "{e}");
    }
}

mod fix_runner {
    use super::conformance::case::{parse_case, Case, Kind};
    use super::conformance::fix::run_fix_text;

    fn fix(body: &str) -> Case {
        parse_case(Kind::Fix, "t", body).unwrap()
    }

    const INPUT: &[u8] = b"(a:N)-->(b:N)\n(c {r: 1..2})\n";
    const FIXED: &[u8] = b"(a:N)-[:K]->(b:N)\n(c {r: 1..2})\n";
    const CASE: &str = "{requires: [], settings: {relType: \"K\"}}\n\
        (:Applied {code: \"rel.no-type\", count: 1})\n\
        (:Remaining {code: \"value.range\", line: 2, column: 8, offset: 25})\n";

    #[test]
    fn a_correct_case_passes() {
        run_fix_text(&fix(CASE), INPUT, FIXED).unwrap();
    }

    #[test]
    fn the_text_is_compared_byte_for_byte() {
        let e = run_fix_text(&fix(CASE), INPUT, b"(a:N)-[:K]->(b:N)\n(c {r: 1..2})").unwrap_err();
        assert!(e.contains("byte"), "{e}");
    }

    #[test]
    fn applied_counts_and_remaining_errors_are_compared() {
        let wrong_count = CASE.replace("count: 1", "count: 2");
        assert!(run_fix_text(&fix(&wrong_count), INPUT, FIXED)
            .unwrap_err()
            .contains("rel.no-type"));
        let missing = CASE.replace("(:Applied {code: \"rel.no-type\", count: 1})\n", "");
        assert!(run_fix_text(&fix(&missing), INPUT, FIXED).is_err());
        let moved = CASE.replace("offset: 25", "offset: 24");
        assert!(run_fix_text(&fix(&moved), INPUT, FIXED)
            .unwrap_err()
            .contains("offset"));
        let none = "{requires: [], settings: {relType: \"K\"}}\n(:Applied {code: \"rel.no-type\", count: 1})\n";
        assert!(run_fix_text(&fix(none), INPUT, FIXED)
            .unwrap_err()
            .contains("value.range"));
    }

    #[test]
    fn without_the_option_the_text_is_left_as_it_was() {
        let c = fix("{requires: []}\n(:Remaining {code: \"rel.no-type\", line: 1, column: 6, offset: 5})\n(:Remaining {code: \"value.range\", line: 2, column: 8, offset: 21})\n");
        run_fix_text(&c, INPUT, INPUT).unwrap();
    }

    #[test]
    fn annotations_are_part_of_the_fixed_text() {
        let c = fix("{requires: [], settings: {annotate: true}}\n(:Remaining {code: \"value.range\", line: 2, column: 7, offset: 0})\n");
        let e = run_fix_text(&c, b"(c {r: 1..2})\n", b"(c {r: 1..2})\n").unwrap_err();
        assert!(e.contains("byte"), "{e}");
    }

    #[test]
    fn a_tag_policy_setting_reaches_the_loader() {
        let c = fix("{requires: [], settings: {tags: \"error\"}}\n(:Applied {code: \"value.tag\", count: 1})\n");
        run_fix_text(&c, b"(a {s: md`x`})", b"(a {s: `x`})").unwrap();
    }
}
