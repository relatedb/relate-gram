//! `check`: parse and plan a file without a database.
use relate_gram::diagnostics::code;
use relate_gram::settings::LoadSettings;

fn check(text: &str) -> relate_gram::CheckReport {
    relate_gram::check(text.as_bytes(), &LoadSettings::default())
}

#[test]
fn a_valid_file_has_no_errors() {
    let r = check("(a:P {name: \"x\"})-[:R]->(b:P)");
    assert!(!r.has_errors(), "{:?}", r.diagnostics);
    assert!(r.diagnostics.is_empty());
}

#[test]
fn errors_are_reported_with_positions_like_a_load() {
    let r = check("(a:N)-->(b:N)\n(c:N)--(d:N)");
    assert!(r.has_errors());
    let found: Vec<(&str, usize)> = r
        .diagnostics
        .iter()
        .map(|d| (d.code, d.span.unwrap().line))
        .collect();
    assert_eq!(
        found,
        vec![(code::REL_NO_TYPE, 1), (code::REL_UNDIRECTED, 2)]
    );
}

#[test]
fn syntax_errors_are_reported() {
    let r = check("(a {k: })");
    assert!(r.has_errors());
    assert_eq!(r.diagnostics[0].code, code::PARSE_MISSING);
}

#[test]
fn warnings_do_not_make_it_fail() {
    let r = check("(a)-[:R]->(b)\n(c {s: md`x`})");
    assert!(!r.has_errors());
    let codes: Vec<&str> = r.diagnostics.iter().map(|d| d.code).collect();
    assert!(codes.contains(&code::ID_REFERENCE_ONLY), "{codes:?}");
    assert!(codes.contains(&code::VALUE_TAG_DROPPED), "{codes:?}");
}

#[test]
fn the_error_tag_policy_applies() {
    let st = LoadSettings {
        tags: relate_gram::settings::TagPolicy::Error,
        ..LoadSettings::default()
    };
    let r = relate_gram::check(b"(a {s: md`x`})", &st);
    assert!(r.has_errors());
}

#[test]
fn check_agrees_with_load_on_every_invalid_fixture() {
    let dir = format!("{}/tests/fixtures/invalid", env!("CARGO_MANIFEST_DIR"));
    let mut n = 0;
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.extension().is_some_and(|x| x == "gram") {
            let bytes = std::fs::read(&p).unwrap();
            assert!(
                relate_gram::check(&bytes, &LoadSettings::default()).has_errors(),
                "{}",
                p.display()
            );
            n += 1;
        }
    }
    assert!(n > 30);
}

#[test]
fn check_reports_every_bad_typed_value_without_a_database() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/invalid");
    let mut n = 0;
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        if !(name.starts_with("tag-content-") && name.ends_with(".gram")) {
            continue;
        }
        let source = std::fs::read(&path).unwrap();
        let r = relate_gram::check(&source, &LoadSettings::default());
        assert!(
            r.diagnostics
                .iter()
                .any(|d| d.code == code::VALUE_TAG_CONTENT),
            "{name}: {:?}",
            r.diagnostics
        );
        n += 1;
    }
    assert!(n >= 20, "expected the tag-content fixtures, found {n}");
}
