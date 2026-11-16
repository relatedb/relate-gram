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
