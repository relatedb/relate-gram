//! The fixer (ADR-010): corrections applied to the text of rejected files.
use relate_gram::fix::{fix, Direction, FixOptions, OctalAs};
use relate_gram::settings::LoadSettings;

fn opts() -> FixOptions {
    FixOptions::default()
}

fn typed(t: &str) -> FixOptions {
    FixOptions {
        rel_type: Some(t.to_string()),
        ..FixOptions::default()
    }
}

/// Fix `src` and return the text. Also checks that fixing the result changes nothing.
fn run(src: &str, settings: &LoadSettings, o: &FixOptions) -> relate_gram::fix::FixOutcome {
    let out = fix(src.as_bytes(), settings, o).expect("fix");
    let again = fix(&out.text, settings, o).expect("fix again");
    assert_eq!(
        again.text, out.text,
        "running the fixer on its own output must change nothing"
    );
    assert!(again.applied.is_empty());
    out
}

fn fixed(src: &str, o: &FixOptions) -> String {
    String::from_utf8(run(src, &LoadSettings::default(), o).text).unwrap()
}

fn errors(out: &relate_gram::fix::FixOutcome) -> Vec<&'static str> {
    out.remaining
        .iter()
        .filter(|d| d.is_error())
        .map(|d| d.code)
        .collect()
}

#[test]
fn a_direction_and_a_type_are_applied_in_rounds() {
    let o = FixOptions {
        rel_type: Some("K".into()),
        direction: Some(Direction::Right),
        ..FixOptions::default()
    };
    for (src, want) in [
        ("(a:N)--(b:N)", "(a:N)-[:K]->(b:N)"),
        ("(a:N)<-->(b:N)", "(a:N)-[:K]->(b:N)"),
    ] {
        let out = run(src, &LoadSettings::default(), &o);
        assert_eq!(String::from_utf8(out.text.clone()).unwrap(), want);
        assert_eq!(out.rounds, 2, "{src}");
        assert!(errors(&out).is_empty());
    }
    let left = FixOptions {
        direction: Some(Direction::Left),
        ..o
    };
    assert_eq!(fixed("(a:N)--(b:N)", &left), "(a:N)<-[:K]-(b:N)");
}

#[test]
fn fat_and_wavy_arrows_become_thin_ones() {
    assert_eq!(fixed("(a:N)=[:T]=>(b:N)", &opts()), "(a:N)-[:T]->(b:N)");
    assert_eq!(fixed("(a:N)~[:T]~>(b:N)", &opts()), "(a:N)-[:T]->(b:N)");
    assert_eq!(fixed("(a:N)<=[:T]=(b:N)", &opts()), "(a:N)<-[:T]-(b:N)");
    // the text inside the brackets is not touched
    assert_eq!(
        fixed("(a:N)=[:T {s: \"a=b~c\"}]=>(b:N)", &opts()),
        "(a:N)-[:T {s: \"a=b~c\"}]->(b:N)"
    );
    // a bare fat arrow still needs a type, which takes a second round
    let out = run("(a:N)==>(b:N)", &LoadSettings::default(), &typed("K"));
    assert_eq!(String::from_utf8(out.text).unwrap(), "(a:N)-[:K]->(b:N)");
    assert_eq!(out.rounds, 2);
    assert_eq!(fixed("(a:N)~~>(b:N)", &typed("K")), "(a:N)-[:K]->(b:N)");
}

#[test]
fn a_file_with_no_errors_is_unchanged() {
    let src = "(a:N)-[:T]->(b:N)\n";
    let out = run(src, &LoadSettings::default(), &typed("K"));
    assert_eq!(out.text, src.as_bytes());
    assert!(out.applied.is_empty());
    assert_eq!(out.rounds, 0);
}

#[test]
fn a_file_with_every_kind_of_error_loads_after_fixing() {
    let src = "// a looser style\n\
               (a::P {o: 010, xs: [1, 2.5]})\n\
               (b:P)\n\
               (a)-->(b)\n\
               (b)--(a)\n\
               (a)=[:L]=>(b)\n\
               (b)<-[:L]->(a)\n";
    let o = FixOptions {
        rel_type: Some("KNOWS".into()),
        direction: Some(Direction::Right),
        octal: Some(OctalAs::Decimal),
        ..FixOptions::default()
    };
    let out = run(src, &LoadSettings::default(), &o);
    assert!(errors(&out).is_empty(), "{:?}", out.remaining);
    let text = String::from_utf8(out.text.clone()).unwrap();
    assert!(text.contains("(a:P {o: 10, xs: [1.0, 2.5]})"), "{text}");
    assert!(
        text.contains("(a)-[:KNOWS]->(b)\n(b)-[:KNOWS]->(a)\n(a)-[:L]->(b)\n(b)-[:L]->(a)"),
        "{text}"
    );
    // the result passes the same check a load runs
    assert!(!relate_gram::check(&out.text, &LoadSettings::default()).has_errors());
    assert!(out.applied.len() >= 7);
    assert!(out.rounds >= 2);
}

#[test]
fn the_applied_fixes_name_their_code() {
    let out = run(
        "(a:N)-->(b:N)\n(c:N)-->(d:N)",
        &LoadSettings::default(),
        &typed("K"),
    );
    assert_eq!(out.applied.len(), 2);
    assert!(out.applied.iter().all(|f| f.code == "rel.no-type"));
    assert!(
        out.applied[0].description.contains('K'),
        "{}",
        out.applied[0].description
    );
}

// --- annotations (ADR-010 section 2.7) ---------------------------------------

fn annotating(o: FixOptions) -> FixOptions {
    FixOptions {
        annotate: true,
        ..o
    }
}

#[test]
fn an_error_that_cannot_be_fixed_gets_a_comment_above_its_line() {
    let src = "(a:N)-->(b:N)\n(c {r: 1..2})\n";
    let out = run(src, &LoadSettings::default(), &annotating(typed("K")));
    let text = String::from_utf8(out.text.clone()).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines[0], "(a:N)-[:K]->(b:N)");
    assert!(
        lines[1]
            .starts_with("// relate-gram: error[value.range]: the range `1..2` has no Cypher type"),
        "{}",
        lines[1]
    );
    assert_eq!(lines[2], "(c {r: 1..2})");
    assert_eq!(out.annotations_added, 1);
    // remaining errors are positioned in the annotated text
    let rest: Vec<_> = out.remaining.iter().filter(|d| d.is_error()).collect();
    assert_eq!(rest.len(), 1);
    assert_eq!(rest[0].span.unwrap().line, 3);
}

#[test]
fn annotations_are_replaced_not_stacked_when_the_fixer_runs_again() {
    let src = "(c {r: 1..2})\n";
    let first = run(src, &LoadSettings::default(), &annotating(opts()));
    assert_eq!(first.annotations_added, 1);
    let second = run(
        std::str::from_utf8(&first.text).unwrap(),
        &LoadSettings::default(),
        &annotating(opts()),
    );
    assert_eq!(second.text, first.text);
    assert_eq!(second.annotations_removed, 1);
    assert_eq!(second.annotations_added, 1);
}

#[test]
fn annotations_of_errors_that_are_now_fixed_are_removed() {
    let first = run(
        "(a:N)-->(b:N)\n",
        &LoadSettings::default(),
        &annotating(opts()),
    );
    let text = String::from_utf8(first.text.clone()).unwrap();
    assert!(
        text.starts_with("// relate-gram: error[rel.no-type]"),
        "{text}"
    );
    // now fix it: the marker goes, and so does the error
    let second = run(&text, &LoadSettings::default(), &annotating(typed("K")));
    assert_eq!(
        String::from_utf8(second.text).unwrap(),
        "(a:N)-[:K]->(b:N)\n"
    );
    assert_eq!(second.annotations_removed, 1);
    assert_eq!(second.annotations_added, 0);
}

#[test]
fn without_annotate_old_markers_are_still_removed_and_none_added() {
    let text = "// relate-gram: error[value.range]: old\n(c {r: 1..2})\n";
    let out = run(text, &LoadSettings::default(), &opts());
    assert_eq!(String::from_utf8(out.text).unwrap(), "(c {r: 1..2})\n");
    assert_eq!(out.annotations_removed, 1);
    assert_eq!(out.annotations_added, 0);
}

#[test]
fn several_errors_on_one_line_each_get_a_marker() {
    let src = "(a {x: 1..2, y: 3..4})\n";
    let out = run(src, &LoadSettings::default(), &annotating(opts()));
    let text = String::from_utf8(out.text).unwrap();
    assert_eq!(text.matches("// relate-gram:").count(), 2);
    assert!(text.ends_with("(a {x: 1..2, y: 3..4})\n"));
    assert_eq!(out.annotations_added, 2);
}

#[test]
fn markers_are_comments_so_a_file_loads_the_same_with_or_without_them() {
    let src = "(a:N)-->(b:N)\n(c {r: 1..2})\n";
    let with = run(src, &LoadSettings::default(), &annotating(opts()));
    let a = relate_gram::check(src.as_bytes(), &LoadSettings::default());
    let b = relate_gram::check(&with.text, &LoadSettings::default());
    let codes =
        |r: &relate_gram::CheckReport| r.diagnostics.iter().map(|d| d.code).collect::<Vec<_>>();
    assert_eq!(codes(&a), codes(&b));
}

// --- typed tags (ADR-011 section 4) ------------------------------------------
