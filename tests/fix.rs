//! The fixer (ADR-010): corrections applied to the text of rejected files.
use relate_gram::fix::{fix, Direction, FixOptions, OctalAs};
use relate_gram::settings::{LoadSettings, TagPolicy};

fn opts() -> FixOptions {
    FixOptions::default()
}

fn typed(t: &str) -> FixOptions {
    FixOptions {
        rel_type: Some(t.to_string()),
        ..FixOptions::default()
    }
}

fn facing(d: Direction) -> FixOptions {
    FixOptions {
        direction: Some(d),
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
fn a_missing_relationship_type_is_inserted_in_every_form() {
    let o = typed("KNOWS");
    assert_eq!(fixed("(a:N)-->(b:N)", &o), "(a:N)-[:KNOWS]->(b:N)");
    assert_eq!(fixed("(a:N)<--(b:N)", &o), "(a:N)<-[:KNOWS]-(b:N)");
    assert_eq!(fixed("(a:N)-[]->(b:N)", &o), "(a:N)-[:KNOWS]->(b:N)");
    assert_eq!(
        fixed("(a:N)-[{since: 1}]->(b:N)", &o),
        "(a:N)-[:KNOWS {since: 1}]->(b:N)"
    );
    assert_eq!(
        fixed("(a:N)-[r {since: 1}]->(b:N)", &o),
        "(a:N)-[r:KNOWS {since: 1}]->(b:N)"
    );
    assert_eq!(
        fixed("(a:N)-->(b:N)-->(c:N)", &o),
        "(a:N)-[:KNOWS]->(b:N)-[:KNOWS]->(c:N)"
    );
}

#[test]
fn a_type_that_needs_quoting_is_quoted() {
    assert_eq!(
        fixed("(a:N)-->(b:N)", &typed("my type")),
        "(a:N)-[:`my type`]->(b:N)"
    );
}

#[test]
fn without_a_type_the_error_is_left_alone() {
    let src = "(a:N)-->(b:N)";
    let out = run(src, &LoadSettings::default(), &opts());
    assert_eq!(out.text, src.as_bytes());
    assert!(out.applied.is_empty());
    assert_eq!(errors(&out), vec!["rel.no-type"]);
}

#[test]
fn an_undirected_arrow_gets_the_chosen_direction() {
    assert_eq!(
        fixed("(a:N)-[:T]-(b:N)", &facing(Direction::Right)),
        "(a:N)-[:T]->(b:N)"
    );
    assert_eq!(
        fixed("(a:N)-[:T]-(b:N)", &facing(Direction::Left)),
        "(a:N)<-[:T]-(b:N)"
    );
    assert_eq!(
        fixed("(a:N)-[r:T {k: 1}]-(b:N)", &facing(Direction::Right)),
        "(a:N)-[r:T {k: 1}]->(b:N)"
    );
}

#[test]
fn a_bidirectional_arrow_loses_one_head() {
    assert_eq!(
        fixed("(a:N)<-[:T]->(b:N)", &facing(Direction::Right)),
        "(a:N)-[:T]->(b:N)"
    );
    assert_eq!(
        fixed("(a:N)<-[:T]->(b:N)", &facing(Direction::Left)),
        "(a:N)<-[:T]-(b:N)"
    );
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
fn a_double_colon_becomes_a_single_colon() {
    assert_eq!(fixed("(a::L)", &opts()), "(a:L)");
    assert_eq!(fixed("(a {k:: 1})", &opts()), "(a {k: 1})");
}

#[test]
fn an_octal_literal_is_replaced_by_the_chosen_reading() {
    let dec = FixOptions {
        octal: Some(OctalAs::Decimal),
        ..FixOptions::default()
    };
    let oct = FixOptions {
        octal: Some(OctalAs::Octal),
        ..FixOptions::default()
    };
    assert_eq!(fixed("(a {o: 010})", &dec), "(a {o: 10})");
    assert_eq!(fixed("(a {o: 010})", &oct), "(a {o: 8})");
    assert_eq!(fixed("(a {o: 00})", &dec), "(a {o: 0})");
    assert_eq!(fixed("(a {o: 00})", &oct), "(a {o: 0})");
    // no reading chosen: left alone
    let out = run("(a {o: 010})", &LoadSettings::default(), &opts());
    assert!(out.applied.is_empty());
    assert_eq!(errors(&out), vec!["value.octal"]);
    // a literal that does not fit in 64 bits has no octal reading
    let big = "(a {o: 0777777777777777777777777})";
    let out = run(big, &LoadSettings::default(), &oct);
    assert_eq!(out.text, big.as_bytes());
}

#[test]
fn a_mixed_numeric_array_is_written_with_decimals() {
    assert_eq!(
        fixed("(a {xs: [1, 2.5, 0x2]})", &opts()),
        "(a {xs: [1.0, 2.5, 2.0]})"
    );
    assert_eq!(
        fixed("(a {xs: [2.5, -3]})", &opts()),
        "(a {xs: [2.5, -3.0]})"
    );
    // any other mixture has no fix
    let src = "(a {xs: [1, \"a\"]})";
    let out = run(src, &LoadSettings::default(), &opts());
    assert_eq!(out.text, src.as_bytes());
    assert_eq!(errors(&out), vec!["value.array-mixed"]);
    // an integer a float cannot hold exactly has no fix
    let big = "(a {xs: [9007199254740993, 1.5]})";
    assert_eq!(
        run(big, &LoadSettings::default(), &opts()).text,
        big.as_bytes()
    );
}

#[test]
fn a_tag_is_removed_when_tags_are_errors() {
    let strict = LoadSettings {
        tags: TagPolicy::Error,
        ..LoadSettings::default()
    };
    let o = opts();
    let out = run("(a {s: md`x`})", &strict, &o);
    assert_eq!(String::from_utf8(out.text).unwrap(), "(a {s: `x`})");
    let out = run("(a {s: ```md\nbody\n```})", &strict, &o);
    assert_eq!(
        String::from_utf8(out.text).unwrap(),
        "(a {s: ```\nbody\n```})"
    );
    // with the default policy a tag is only a warning, and the file loads: nothing to fix
    let src = "(a {s: md`x`})";
    let out = run(src, &LoadSettings::default(), &o);
    assert_eq!(out.text, src.as_bytes());
    assert!(out.applied.is_empty());
}

#[test]
fn everything_outside_the_edits_is_kept_byte_for_byte() {
    let src =
        "\u{feff}// keep me\r\n(a:N)-->(b:N) // and me\r\n\r\n/** docs */\r\n(c {s: \"x  y\"})\r\n";
    let want = "\u{feff}// keep me\r\n(a:N)-[:K]->(b:N) // and me\r\n\r\n/** docs */\r\n(c {s: \"x  y\"})\r\n";
    assert_eq!(fixed(src, &typed("K")), want);
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
fn a_syntax_error_is_not_fixed_and_is_reported() {
    let src = "(a)-->(b)\n(c {k: })";
    let out = run(src, &LoadSettings::default(), &typed("K"));
    assert_eq!(out.text, src.as_bytes());
    assert!(errors(&out).contains(&"parse.missing"));
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

#[test]
fn remaining_errors_are_positioned_in_the_fixed_text() {
    // the first line's fix lengthens it; the remaining error on line 2 keeps its line
    let out = run(
        "(a:N)-->(b:N)\n(c {r: 1..2})",
        &LoadSettings::default(),
        &typed("KNOWS"),
    );
    let rest: Vec<_> = out.remaining.iter().filter(|d| d.is_error()).collect();
    assert_eq!(rest.len(), 1);
    assert_eq!(rest[0].code, "value.range");
    let text = String::from_utf8(out.text.clone()).unwrap();
    let span = rest[0].span.unwrap();
    assert_eq!(&text[span.start..span.end], "1..2");
}
