//! Spec 006 T002: the pattern-rs gram reader compiles into this crate and reads the shape of a
//! case file (ADR-015 section 2.2): a header record, then labeled nodes.
use gram_codec::parse_gram_with_header;
use pattern_core::Value;

#[test]
fn a_case_file_reads_as_a_header_and_expectation_nodes() {
    let text = "/** why */\n{requires: [], settings: {tags: \"error\"}}\n(:Error {code: \"rel.no-type\", line: 2, column: 4, offset: 31})\n";
    let (header, patterns) = parse_gram_with_header(text).expect("parses");
    let header = header.expect("a header record");
    assert!(matches!(header.get("requires"), Some(Value::VArray(a)) if a.is_empty()));
    assert!(matches!(header.get("settings"), Some(Value::VMap(_))));
    assert_eq!(patterns.len(), 1);
    let s = &patterns[0].value;
    assert!(s.labels.contains("Error"));
    assert_eq!(s.properties.get("line"), Some(&Value::VInteger(2)));
}
