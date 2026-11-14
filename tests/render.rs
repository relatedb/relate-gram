//! Rendering a diagnostic with the source line and a marker under the text.
use relate_gram::diagnostics::{Diagnostic, LineIndex};
use relate_gram::render::render;

fn diag_at(source: &str, start: usize, end: usize, message: &str) -> Diagnostic {
    let idx = LineIndex::new(source.as_bytes(), 0);
    Diagnostic::error("rel.no-type", idx.span(start, end), message)
}

#[test]
fn an_error_shows_its_line_and_a_marker_under_the_text() {
    let src = "(a:N)\n(b:N)-->(c:N)\n";
    let d = diag_at(src, 11, 14, "a relationship needs a type");
    assert_eq!(
        render(&d, src.as_bytes(), "t.gram"),
        "error[rel.no-type]: a relationship needs a type\n\
         \x20--> t.gram:2:6\n\
         \x20 |\n\
         2 | (b:N)-->(c:N)\n\
         \x20 |      ^^^\n"
    );
}

#[test]
fn the_gutter_is_as_wide_as_the_largest_line_number() {
    let mut src = String::new();
    for _ in 0..11 {
        src.push_str("(x:N)\n");
    }
    src.push_str("(a)-->(b)\n");
    let start = src.find("-->").unwrap();
    let d = diag_at(&src, start, start + 3, "m");
    let out = render(&d, src.as_bytes(), "f.gram");
    assert!(out.contains("  --> f.gram:12:4\n"), "{out}");
    assert!(out.contains("12 | (a)-->(b)\n"), "{out}");
    assert!(out.contains("   |    ^^^\n"), "{out}");
}

#[test]
fn a_warning_is_labelled_as_one() {
    let src = "(a {s: md`x`})\n";
    let idx = LineIndex::new(src.as_bytes(), 0);
    let d = Diagnostic::warning("value.tag-dropped", idx.span(7, 12), "the tag was dropped");
    let out = render(&d, src.as_bytes(), "t.gram");
    assert!(
        out.starts_with("warning[value.tag-dropped]: the tag was dropped\n"),
        "{out}"
    );
    assert!(out.contains("  |        ^^^^^\n"), "{out}");
}

#[test]
fn a_related_span_is_shown_with_its_label() {
    let src = "(a:L)\n(a:M)\n";
    let idx = LineIndex::new(src.as_bytes(), 0);
    let d = Diagnostic::error("id.duplicate-definition", idx.span(7, 8), "defined twice")
        .with_related("first defined here", idx.span(1, 2));
    let out = render(&d, src.as_bytes(), "t.gram");
    assert_eq!(
        out,
        "error[id.duplicate-definition]: defined twice\n\
         \x20--> t.gram:2:2\n\
         \x20 |\n\
         2 | (a:M)\n\
         \x20 |  ^\n\
         \x20 |\n\
         1 | (a:L)\n\
         \x20 |  - first defined here\n"
    );
}

#[test]
fn a_diagnostic_without_a_span_is_one_line() {
    let d = Diagnostic::error_without_span("load.database-error", "the database refused it");
    assert_eq!(
        render(&d, b"(a)", "t.gram"),
        "error[load.database-error]: the database refused it\n"
    );
}

#[test]
fn markers_line_up_under_multibyte_text_and_tabs() {
    let src = "(é:N)\t-->(b)\n";
    let start = src.find("-->").unwrap();
    let d = diag_at(src, start, start + 3, "m");
    let out = render(&d, src.as_bytes(), "t.gram");
    // the source line is shown as written; the marker line copies its tab
    assert!(out.contains("1 | (é:N)\t-->(b)\n"), "{out}");
    assert!(out.contains("  |      \t^^^\n"), "{out}");
}

#[test]
fn a_span_that_runs_past_its_first_line_is_marked_to_the_end_of_that_line() {
    let src = "(a {k: ```\nfence\n```})\n";
    let idx = LineIndex::new(src.as_bytes(), 0);
    let d = Diagnostic::error("value.tag", idx.span(7, 21), "m");
    let out = render(&d, src.as_bytes(), "t.gram");
    assert!(out.contains("1 | (a {k: ```\n"), "{out}");
    assert!(out.contains("  |        ^^^\n"), "{out}");
}

#[test]
fn input_that_is_not_utf8_and_a_byte_order_mark_are_handled() {
    let bytes = b"(a)\n(\xff)\n";
    let idx = LineIndex::new(bytes, 0);
    let d = Diagnostic::error("input.encoding", idx.span(5, 6), "not valid UTF-8");
    let out = render(&d, bytes, "t.gram");
    assert!(out.contains("2 | (\u{fffd})\n"), "{out}");

    let bom = "\u{feff}(a)-->(b)\n";
    let d = Diagnostic::error(
        "rel.no-type",
        LineIndex::new(&bom.as_bytes()[3..], 3).span(3, 6),
        "m",
    );
    let out = render(&d, bom.as_bytes(), "t.gram");
    assert!(out.contains("1 | (a)-->(b)\n"), "{out}");
    assert!(out.contains("  |    ^^^\n"), "{out}");
}

#[test]
fn a_bad_typed_value_shows_a_marker_under_the_tagged_string() {
    let src = "(a {when: date`2024-13-45`})\n";
    let report = relate_gram::check(
        src.as_bytes(),
        &relate_gram::settings::LoadSettings::default(),
    );
    let d = &report.diagnostics[0];
    assert_eq!(d.code, "value.tag-content");
    let out = render(d, src.as_bytes(), "t.gram");
    assert!(
        out.starts_with("error[value.tag-content]: the content of the `date` tag is not valid"),
        "{out}"
    );
    assert!(out.contains(" --> t.gram:1:11\n"), "{out}");
    assert!(out.contains("1 | (a {when: date`2024-13-45`})\n"), "{out}");
    assert!(out.contains("  |           ^^^^^^^^^^^^^^^^\n"), "{out}");
}
