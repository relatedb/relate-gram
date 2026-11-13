//! Source positions and the diagnostics every load and export reports.
//!
//! Every error and warning has a stable `code` (listed in
//! `specs/001-phase1-load-export/contracts/diagnostics.md`), a severity, a
//! span where the source text is at fault, and a message.

use std::fmt;

/// A range of the source: byte offsets plus the 1-based line and column of the start.
/// The column counts bytes from the start of the line (ADR-007 section 2.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
}

/// Maps byte offsets of a text to lines and columns.
#[derive(Clone, Debug)]
pub struct LineIndex {
    /// Offset of the first byte of each line, relative to the indexed text.
    line_starts: Vec<usize>,
    base: usize,
}

impl LineIndex {
    /// Index `text`. `base` is added to every reported offset (the length of a
    /// byte order mark that was skipped before parsing).
    pub fn new(text: &[u8], base: usize) -> Self {
        let mut line_starts = vec![0];
        for (i, b) in text.iter().enumerate() {
            if *b == b'\n' {
                line_starts.push(i + 1);
            }
        }
        LineIndex { line_starts, base }
    }

    /// The span of `start..end`, given as offsets into the indexed text.
    pub fn span(&self, start: usize, end: usize) -> Span {
        let line = match self.line_starts.binary_search(&start) {
            Ok(i) => i,
            Err(i) => i - 1,
        };
        Span {
            start: start + self.base,
            end: end + self.base,
            line: line + 1,
            column: start - self.line_starts[line] + 1,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

/// One error or warning.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: &'static str,
    pub severity: Severity,
    pub span: Option<Span>,
    pub message: String,
    /// Labelled earlier or related spans, such as the first definition of a duplicate.
    pub related: Vec<(String, Span)>,
}

impl Diagnostic {
    pub fn error(code: &'static str, span: Span, message: impl Into<String>) -> Self {
        Diagnostic {
            code,
            severity: Severity::Error,
            span: Some(span),
            message: message.into(),
            related: Vec::new(),
        }
    }

    pub fn warning(code: &'static str, span: Span, message: impl Into<String>) -> Self {
        Diagnostic {
            code,
            severity: Severity::Warning,
            span: Some(span),
            message: message.into(),
            related: Vec::new(),
        }
    }

    pub fn error_without_span(code: &'static str, message: impl Into<String>) -> Self {
        Diagnostic {
            code,
            severity: Severity::Error,
            span: None,
            message: message.into(),
            related: Vec::new(),
        }
    }

    pub fn warning_without_span(code: &'static str, message: impl Into<String>) -> Self {
        Diagnostic {
            code,
            severity: Severity::Warning,
            span: None,
            message: message.into(),
            related: Vec::new(),
        }
    }

    pub fn with_related(mut self, label: impl Into<String>, span: Span) -> Self {
        self.related.push((label.into(), span));
        self
    }

    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sev = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        match self.span {
            Some(s) => write!(
                f,
                "{sev}[{}] {}:{}: {}",
                self.code, s.line, s.column, self.message
            ),
            None => write!(f, "{sev}[{}]: {}", self.code, self.message),
        }
    }
}

/// The stable diagnostic codes.
pub mod code {
    pub const INPUT_ENCODING: &str = "input.encoding";
    pub const PARSE_ERROR: &str = "parse.error";
    pub const PARSE_MISSING: &str = "parse.missing";
    pub const CONSTRUCT_ANNOTATION: &str = "construct.annotation";
    pub const CONSTRUCT_SUBJECT_PATTERN: &str = "construct.subject-pattern";
    pub const CONSTRUCT_DOUBLE_COLON: &str = "construct.double-colon";
    pub const CONSTRUCT_UNSUPPORTED: &str = "construct.unsupported";
    pub const REL_UNDIRECTED: &str = "rel.undirected";
    pub const REL_BIDIRECTIONAL: &str = "rel.bidirectional";
    pub const REL_ARROW_FAMILY: &str = "rel.arrow-family";
    pub const REL_NO_TYPE: &str = "rel.no-type";
    pub const REL_MANY_LABELS: &str = "rel.many-labels";
    pub const VALUE_RANGE: &str = "value.range";
    pub const VALUE_MEASUREMENT: &str = "value.measurement";
    pub const VALUE_MAP: &str = "value.map";
    pub const VALUE_OCTAL: &str = "value.octal";
    pub const VALUE_INTEGER_RANGE: &str = "value.integer-range";
    pub const VALUE_DECIMAL: &str = "value.decimal";
    pub const VALUE_ARRAY_MIXED: &str = "value.array-mixed";
    pub const VALUE_TAG_DROPPED: &str = "value.tag-dropped";
    pub const VALUE_TAG: &str = "value.tag";
    pub const VALUE_TAG_CONTENT: &str = "value.tag-content";
    pub const KEY_DUPLICATE: &str = "key.duplicate";
    pub const KEY_EMPTY: &str = "key.empty";
    pub const KEY_RESERVED: &str = "key.reserved";
    pub const ID_KIND_CONFLICT: &str = "id.kind-conflict";
    pub const ID_DUPLICATE_DEFINITION: &str = "id.duplicate-definition";
    pub const ID_ENDPOINTS_DIFFER: &str = "id.endpoints-differ";
    pub const ID_REL_NEVER_DEFINED: &str = "id.rel-never-defined";
    pub const ID_REFERENCE_ONLY: &str = "id.reference-only";
    pub const ID_PROPERTY_MISSING: &str = "id.property-missing";
    pub const MATCH_KIND_DIFFERS: &str = "match.kind-differs";
    pub const MATCH_TYPE_DIFFERS: &str = "match.type-differs";
    pub const MATCH_DB_DUPLICATE: &str = "match.db-duplicate";
    pub const FILE_KIND: &str = "file.kind";
    pub const FILE_RECORD_NOT_STORED: &str = "file.record-not-stored";
    pub const LOAD_ANONYMOUS: &str = "load.anonymous";
    pub const LOAD_IN_TRANSACTION: &str = "load.in-transaction";
    pub const LOAD_DATABASE_ERROR: &str = "load.database-error";
    pub const EXPORT_CONFIG: &str = "export.config";
    pub const EXPORT_ID_MISSING: &str = "export.id-missing";
    pub const EXPORT_ID_UNUSABLE: &str = "export.id-unusable";
    pub const EXPORT_ID_DUPLICATE: &str = "export.id-duplicate";
    pub const EXPORT_VALUE: &str = "export.value";
    pub const EXPORT_KEY_RESERVED: &str = "export.key-reserved";
    pub const EXPORT_IN_TRANSACTION: &str = "export.in-transaction";

    /// Every code, for tests that check coverage.
    pub const ALL: &[&str] = &[
        INPUT_ENCODING,
        PARSE_ERROR,
        PARSE_MISSING,
        CONSTRUCT_ANNOTATION,
        CONSTRUCT_SUBJECT_PATTERN,
        CONSTRUCT_DOUBLE_COLON,
        CONSTRUCT_UNSUPPORTED,
        REL_UNDIRECTED,
        REL_BIDIRECTIONAL,
        REL_ARROW_FAMILY,
        REL_NO_TYPE,
        REL_MANY_LABELS,
        VALUE_RANGE,
        VALUE_MEASUREMENT,
        VALUE_MAP,
        VALUE_OCTAL,
        VALUE_INTEGER_RANGE,
        VALUE_DECIMAL,
        VALUE_ARRAY_MIXED,
        VALUE_TAG_DROPPED,
        VALUE_TAG,
        VALUE_TAG_CONTENT,
        KEY_DUPLICATE,
        KEY_EMPTY,
        KEY_RESERVED,
        ID_KIND_CONFLICT,
        ID_DUPLICATE_DEFINITION,
        ID_ENDPOINTS_DIFFER,
        ID_REL_NEVER_DEFINED,
        ID_REFERENCE_ONLY,
        ID_PROPERTY_MISSING,
        MATCH_KIND_DIFFERS,
        MATCH_TYPE_DIFFERS,
        MATCH_DB_DUPLICATE,
        FILE_KIND,
        FILE_RECORD_NOT_STORED,
        LOAD_ANONYMOUS,
        LOAD_IN_TRANSACTION,
        LOAD_DATABASE_ERROR,
        EXPORT_CONFIG,
        EXPORT_ID_MISSING,
        EXPORT_ID_UNUSABLE,
        EXPORT_ID_DUPLICATE,
        EXPORT_VALUE,
        EXPORT_KEY_RESERVED,
        EXPORT_IN_TRANSACTION,
    ];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn span_lf_lines_and_columns() {
        let text = b"(a)\n(b)\n";
        let idx = LineIndex::new(text, 0);
        let s = idx.span(5, 6); // the `b`
        assert_eq!((s.line, s.column, s.start, s.end), (2, 2, 5, 6));
        let first = idx.span(0, 1);
        assert_eq!((first.line, first.column), (1, 1));
    }

    #[test]
    fn span_crlf_counts_cr_on_previous_line() {
        let text = b"(a)\r\n(b)";
        let idx = LineIndex::new(text, 0);
        // offset of `(` of the second line is 5
        let s = idx.span(5, 6);
        assert_eq!((s.line, s.column), (2, 1));
        // the `\r` itself is column 4 of line 1
        let cr = idx.span(3, 4);
        assert_eq!((cr.line, cr.column), (1, 4));
    }

    #[test]
    fn span_columns_count_bytes_not_characters() {
        let text = "(é x)".as_bytes(); // `é` is two bytes
        let idx = LineIndex::new(text, 0);
        let s = idx.span(4, 5); // `x`
        assert_eq!((s.line, s.column), (1, 5));
    }

    #[test]
    fn span_base_offset_shifts_offsets_not_columns() {
        let idx = LineIndex::new(b"(a)", 3);
        let s = idx.span(1, 2);
        assert_eq!((s.start, s.end, s.line, s.column), (4, 5, 1, 2));
    }

    #[test]
    fn span_at_end_of_input() {
        let idx = LineIndex::new(b"(a)\n", 0);
        let s = idx.span(4, 4);
        assert_eq!((s.line, s.column), (2, 1));
    }

    #[test]
    fn diagnostic_constructors() {
        let idx = LineIndex::new(b"(a)", 0);
        let sp = idx.span(0, 1);
        let e = Diagnostic::error(code::PARSE_ERROR, sp, "bad");
        assert_eq!(e.severity, Severity::Error);
        assert_eq!(e.span, Some(sp));
        let w = Diagnostic::warning(code::VALUE_TAG_DROPPED, sp, "tag").with_related("first", sp);
        assert_eq!(w.severity, Severity::Warning);
        assert_eq!(w.related.len(), 1);
        let n = Diagnostic::error_without_span(code::LOAD_DATABASE_ERROR, "db");
        assert_eq!(n.span, None);
        assert!(e.to_string().contains("1:1"));
        assert!(e.to_string().contains("parse.error"));
    }
}
