//! The gaps SPIKE-006 found in the pattern-rs libraries, checked against the typed layer.
use gram_value::{Kind, Value};
use gramsyn::*;

fn input(name: &str) -> Document {
    let path = format!(
        "{}/../../SPIKE-006-pattern-rs-readers/inputs/{name}.gram",
        env!("CARGO_MANIFEST_DIR")
    );
    parse(&std::fs::read_to_string(path).unwrap())
}

fn codes(d: &Document) -> Vec<&'static str> {
    d.diagnostics.iter().map(|x| x.code).collect()
}

fn first_record(d: &Document, i: usize) -> &Record {
    match &d.elements[i] {
        Element::Node { subject: Some(s), .. } => s.record.as_ref().unwrap(),
        other => panic!("{other:?}"),
    }
}

#[test]
fn the_case_file_shape_reads_with_positions() {
    let d = input("ok-01-case-file");
    assert!(d.diagnostics.is_empty(), "{:?}", d.diagnostics);
    let h = d.record.as_ref().expect("header record");
    let kinds: Vec<(&str, Kind)> = h
        .properties
        .iter()
        .map(|p| (p.key.text.as_str(), p.value.value.kind()))
        .collect();
    assert_eq!(kinds, [("kind", Kind::String), ("requires", Kind::Array), ("settings", Kind::Map)]);
    assert_eq!(d.doc_comments.len(), 1);
    // every value knows where it is
    for p in &h.properties {
        assert!(!d.text(p.value.span).is_empty());
    }
    let (line, col) = d.line_col(h.properties[0].value.span.start);
    assert_eq!((line, col), (2, 8));
}

#[test]
fn repeated_keys_are_kept_and_reported_once_each() {
    let d = input("bad-02-duplicate-key");
    assert_eq!(codes(&d), ["duplicate-key"]);
    let r = first_record(&d, 0);
    assert_eq!(r.properties.len(), 2, "both properties are kept in written order");
    let diag = &d.diagnostics[0];
    assert_eq!(d.text(diag.span), "k");
    assert!(diag.span.start > diag.related.unwrap().1.start);
    let h = input("bad-03-duplicate-header-key");
    assert_eq!(codes(&h), ["duplicate-key"]);
}

#[test]
fn a_fenced_tag_is_kept() {
    let d = input("ok-11-fenced-tag");
    let r = first_record(&d, 0);
    let Value::Tagged(t) = &r.properties[0].value.value else {
        panic!("{:?}", r.properties[0].value.value)
    };
    assert_eq!((t.tag(), t.content()), ("date", "2024-01-15\n"));
}

#[test]
fn arrow_family_and_direction_are_kept() {
    let d = input("ok-12-arrow-families");
    let got: Vec<(ArrowFamily, Direction)> = d
        .elements
        .iter()
        .map(|e| match e {
            Element::Relationship { arrow, .. } => (arrow.family, arrow.direction),
            other => panic!("{other:?}"),
        })
        .collect();
    use ArrowFamily::*;
    use Direction::*;
    assert_eq!(
        got,
        [(Equals, Right), (Tilde, Right), (Dash, Undirected), (Dash, Both), (Dash, Right)]
    );
}

#[test]
fn double_colon_labels_are_read_and_marked() {
    let d = input("ok-15a-double-colon");
    assert!(d.diagnostics.is_empty());
    let Element::Node { subject: Some(s), .. } = &d.elements[0] else { panic!() };
    assert!(s.labels[0].double_colon);
    let d = input("ok-22-double-colon-record");
    assert!(d.record.unwrap().properties[0].double_colon);
}

#[test]
fn big_numbers_are_held_not_rejected() {
    let d = input("ok-08-beyond-i64");
    assert!(d.diagnostics.is_empty(), "{:?}", d.diagnostics);
    assert_eq!(first_record(&d, 0).properties[0].value.value.kind(), Kind::BigInteger);
    let d = input("ok-09-long-decimal");
    assert_eq!(first_record(&d, 0).properties[0].value.value.kind(), Kind::BigDecimal);
}

#[test]
fn an_annotation_keeps_a_map_value_as_a_map() {
    let d = input("ok-07-annotations");
    assert!(d.diagnostics.is_empty(), "{:?}", d.diagnostics);
    let Element::Annotated { annotations, .. } = &d.elements[2] else { panic!() };
    let Value::Map(m) = &annotations.properties[0].value.value else { panic!() };
    assert_eq!(m.get("line"), Some(&Value::integer(2)));
    assert_eq!(m.len(), 2);
}

#[test]
fn an_empty_record_is_a_record() {
    let d = input("ok-19-empty-record");
    assert_eq!(d.record.as_ref().unwrap().properties.len(), 0);
    assert_eq!(d.elements.len(), 1);
}

#[test]
fn malformed_input_is_reported_with_every_error() {
    for name in [
        "bad-01-missing-value",
        "bad-04-unterminated",
        "bad-05-block-comment",
        "bad-06-top-level-comma",
        "bad-07-quoted-identifier",
        "bad-08-stray-token",
    ] {
        let d = input(name);
        assert!(
            d.diagnostics.iter().any(|x| matches!(x.code, "syntax-error" | "missing-node")),
            "{name}: {:?}",
            d.diagnostics
        );
    }
}

#[test]
fn every_ok_input_has_no_diagnostics_except_the_known_tag_content_one() {
    let dir = format!("{}/../../SPIKE-006-pattern-rs-readers/inputs", env!("CARGO_MANIFEST_DIR"));
    let mut names: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
        .filter(|n| n.starts_with("ok-"))
        .collect();
    names.sort();
    for n in names {
        let d = input(n.trim_end_matches(".gram"));
        // ok-11 writes a fenced date with a trailing newline, which gram-value reports.
        let bad: Vec<_> = d.diagnostics.iter().filter(|x| x.code != "invalid-form").collect();
        assert!(bad.is_empty(), "{n}: {bad:?}");
    }
}

#[test]
fn paths_project_as_gram_documents_them_and_direction_orders_elements() {
    let d = input("ok-05-paths");
    let (patterns, diags) = project(&d);
    assert!(diags.is_empty());
    // (a)-[:R {w: 1}]->(b)-[:S]->(c)  is  [ | [R | a, b], [S | b, c] ]
    let path = &patterns[0];
    assert_eq!(path.subject, PatternSubject::default());
    assert_eq!(path.elements.len(), 2);
    let ids = |p: &Pattern| -> Vec<String> {
        p.elements.iter().map(|e| e.subject.identity.clone().unwrap()).collect()
    };
    assert_eq!(path.elements[0].subject.labels, ["R"]);
    assert_eq!(ids(&path.elements[0]), ["a", "b"]);
    assert_eq!(path.elements[1].subject.labels, ["S"]);
    assert_eq!(ids(&path.elements[1]), ["b", "c"]);
    // (x)<-[:T]-(y)  is  [T | y, x]
    assert_eq!(ids(&patterns[1]), ["y", "x"]);
}

#[test]
fn an_element_defined_twice_is_reported_at_the_second_with_the_first_related() {
    let d = parse("(a:X)\n(b)\n(a:Y)\n(a)\n");
    let (_, diags) = project(&d);
    assert_eq!(diags.len(), 1, "{diags:?}");
    let diag = &diags[0];
    assert_eq!(diag.code, "duplicate-definition");
    assert_eq!(d.line_col(diag.span.start), (3, 2), "the identifier, not the pattern");
    assert_eq!(d.line_col(diag.related.unwrap().1.start), (1, 2));
}

#[test]
fn an_annotation_projects_as_a_one_element_pattern_with_the_annotation_as_subject() {
    let d = input("ok-07-annotations");
    let (patterns, _) = project(&d);
    assert_eq!(patterns[0].elements.len(), 1);
    assert_eq!(patterns[0].subject.properties[0].0, "desc");
    assert_eq!(patterns[1].subject.identity.as_deref(), Some("id1"));
}
