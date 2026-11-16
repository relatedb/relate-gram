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
    assert!(matches!(&c.expectations[2], Expectation::Counts(m) if m["nodesCreated"] == 2 && m["total"] == 7));
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
    let e = parse_case(Kind::Load, "x", "{requires: []}\n(:Counts {nodesMade: 1})\n").unwrap_err();
    assert!(e.contains("nodesMade"), "{e}");
}

#[test]
fn an_error_node_needs_a_code_and_all_or_none_of_the_position() {
    assert!(parse_case(Kind::Reject, "x", "{requires: []}\n(:Error {line: 1})\n").is_err());
    assert!(parse_case(Kind::Reject, "x", "{requires: []}\n(:Error {code: \"a\", line: 1})\n").is_err());
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
    assert!(read_case(&dir, Kind::Reject, "x").unwrap_err().contains("input"));
    write("input.txt", "(a)\n");
    assert!(read_case(&dir, Kind::Reject, "x").is_ok());
    // reject does not use expected.gram
    write("expected.gram", "(a)\n");
    assert!(read_case(&dir, Kind::Reject, "x").unwrap_err().contains("expected.gram"));
    // load needs expected.gram and input.txt
    assert!(read_case(&dir, Kind::Load, "x").is_ok());
    std::fs::remove_file(dir.join("expected.gram")).unwrap();
    assert!(read_case(&dir, Kind::Load, "x").unwrap_err().contains("expected.gram"));
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
    assert!(e.contains("is not a setting of an export"), "an export does not take a fix option: {e}");
    let c2 = parse_case(Kind::Export, "x", "{requires: [], settings: {identity: \"property\", nodeIdProperty: \"k\"}}\n").unwrap();
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
    let c = parse_case(Kind::Load, "x", "{requires: [], settings: {tagz: \"error\"}}\n").unwrap();
    assert!(settings::load_settings(&c.settings).unwrap_err().contains("tagz"));
    let c = parse_case(Kind::Load, "x", "{requires: [], settings: {maxDiagnostics: \"five\"}}\n").unwrap();
    assert!(settings::load_settings(&c.settings).unwrap_err().contains("maxDiagnostics"));
    let c = parse_case(Kind::Load, "x", "{requires: [], settings: {tags: \"loud\"}}\n").unwrap();
    assert!(settings::load_settings(&c.settings).unwrap_err().contains("tags"));
}
