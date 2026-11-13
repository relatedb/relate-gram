use relate_gram::__test_support::parse;
use relate_gram::diagnostics::code;

fn fail(input: &[u8], max: usize) -> (Vec<relate_gram::diagnostics::Diagnostic>, usize) {
    match parse(input, max) {
        Ok(_) => panic!("expected failure for {:?}", String::from_utf8_lossy(input)),
        Err(f) => (f.diagnostics, f.total),
    }
}

#[test]
fn valid_input_parses() {
    assert!(parse(b"(a)-[:R]->(b)", 100).is_ok());
}

#[test]
fn non_utf8_is_an_error_at_the_first_bad_byte() {
    let (d, total) = fail(b"(a)\n(\xff)", 100);
    assert_eq!(total, 1);
    assert_eq!(d[0].code, code::INPUT_ENCODING);
    let s = d[0].span.unwrap();
    assert_eq!((s.line, s.column, s.start), (2, 2, 5));
}

#[test]
fn byte_order_mark_is_skipped_and_offsets_include_it() {
    assert!(parse("\u{feff}(a)".as_bytes(), 100).is_ok());
    // `(a` is unclosed: the error is reported on line 1, offsets count the 3 BOM bytes
    let (d, _) = fail("\u{feff}(a {k: })".as_bytes(), 100);
    let s = d[0].span.unwrap();
    assert_eq!(s.line, 1);
    assert!(s.start >= 3 + 4, "offset must include the BOM: {s:?}");
    assert!(s.column >= 5, "column excludes the BOM: {s:?}");
}

#[test]
fn crlf_line_endings_give_correct_lines() {
    assert!(parse(b"(a)\r\n(b)\r\n", 100).is_ok());
    let (d, _) = fail(b"(a)\r\n(b {k: })", 100);
    let s = d[0].span.unwrap();
    assert_eq!(s.line, 2);
}

#[test]
fn missing_value_is_parse_missing_with_expected_kind() {
    let (d, _) = fail(b"(a {k: })", 100);
    let m = d
        .iter()
        .find(|x| x.code == code::PARSE_MISSING)
        .expect("a MISSING diagnostic");
    assert!(
        m.message.contains("symbol"),
        "message names the expected kind: {}",
        m.message
    );
    let s = m.span.unwrap();
    assert_eq!(s.line, 1);
}

#[test]
fn empty_array_parses() {
    assert!(parse(b"(a {xs: []})", 100).is_ok());
    assert!(parse(b"(a {xs: [ ]})", 100).is_ok());
}

#[test]
fn array_of_empty_arrays_is_an_error() {
    let (d, _) = fail(b"(a {xs: [[]]})", 100);
    assert!(d
        .iter()
        .any(|x| x.code == code::PARSE_ERROR || x.code == code::PARSE_MISSING));
}

#[test]
fn raw_newline_in_quoted_string_is_a_parse_error() {
    let (d, _) = fail(b"(a {s: \"x\ny\"})", 100);
    assert!(d.iter().any(|x| x.code == code::PARSE_ERROR));
}

#[test]
fn every_error_is_collected_in_source_order_and_the_cap_applies() {
    let input = b"(a {k: })\n(b {k: })\n(c {k: })\n";
    let (d, total) = fail(input, 100);
    assert!(total >= 3);
    let starts: Vec<usize> = d.iter().map(|x| x.span.unwrap().start).collect();
    let mut sorted = starts.clone();
    sorted.sort();
    assert_eq!(starts, sorted);
    let (capped, total2) = fail(input, 2);
    assert_eq!(capped.len(), 2);
    assert_eq!(total2, total);
}
