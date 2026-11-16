//! The typed syntax model.
use gram_value::Value;

/// A byte range of the source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

/// How a name was written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameStyle {
    Symbol,
    Integer,
    Backtick,
    DoubleQuote,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Name {
    pub text: String,
    pub span: Span,
    pub style: NameStyle,
}

/// A value with its position, the grammar node it came from, and the positions of its parts
/// (the elements of an array, or the values of a map's entries).
#[derive(Debug, Clone, PartialEq)]
pub struct Lowered {
    pub value: Value,
    pub span: Span,
    /// The grammar node kind: `integer`, `octal`, `hexadecimal`, `string_literal`, `symbol`,
    /// `tagged_string`, `measurement`, `range`, `array`, `map`, ...
    pub node_kind: &'static str,
    pub parts: Vec<Span>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Property {
    pub span: Span,
    pub key: Name,
    /// `::` instead of `:` after the key.
    pub double_colon: bool,
    /// The `:` or `::` token.
    pub colon_span: Span,
    pub value: Lowered,
}

/// A record, in written order, with repeated keys kept.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Record {
    pub span: Option<Span>,
    pub properties: Vec<Property>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Label {
    pub name: Name,
    pub double_colon: bool,
    /// The `:` or `::` token.
    pub colon_span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Subject {
    pub span: Span,
    pub identifier: Option<Name>,
    pub labels: Vec<Label>,
    pub record: Option<Record>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrowFamily {
    Dash,
    Equals,
    Tilde,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Undirected,
    Right,
    Left,
    Both,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Arrow {
    pub span: Span,
    pub family: ArrowFamily,
    pub direction: Direction,
    pub subject: Option<Subject>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnnotationProperty {
    pub span: Span,
    pub key: Name,
    pub value: Lowered,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Annotations {
    pub span: Span,
    /// `@@id:Label`, if present.
    pub identified: Option<(Option<Name>, Vec<Label>)>,
    pub properties: Vec<AnnotationProperty>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Element {
    Node {
        span: Span,
        subject: Option<Subject>,
    },
    Relationship {
        span: Span,
        left: Box<Element>,
        arrow: Arrow,
        right: Box<Element>,
    },
    SubjectPattern {
        span: Span,
        subject: Option<Subject>,
        elements: Vec<Element>,
    },
    Annotated {
        span: Span,
        annotations: Annotations,
        element: Box<Element>,
    },
    Reference {
        span: Span,
        identifier: Name,
    },
}

impl Element {
    pub fn span(&self) -> Span {
        match self {
            Element::Node { span, .. }
            | Element::Relationship { span, .. }
            | Element::SubjectPattern { span, .. }
            | Element::Annotated { span, .. }
            | Element::Reference { span, .. } => *span,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

/// A diagnostic about gram itself. `path` is gram-value's path into a value, when the problem
/// is inside one.
#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub code: &'static str,
    pub severity: Severity,
    pub span: Span,
    pub message: String,
    pub path: Vec<gram_value::PathSegment>,
    pub related: Option<(&'static str, Span)>,
    /// The grammar node kind the problem is in (`tagged_string`, `hexadecimal`, ...), or empty.
    pub node_kind: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DocComment {
    pub span: Span,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct Document {
    pub source: String,
    pub record: Option<Record>,
    pub elements: Vec<Element>,
    pub doc_comments: Vec<DocComment>,
    pub diagnostics: Vec<Diagnostic>,
    pub(crate) line_starts: Vec<usize>,
}

impl Document {
    /// The 1-based line and column of a byte offset; the column counts bytes.
    pub fn line_col(&self, offset: usize) -> (usize, usize) {
        let line = self.line_starts.partition_point(|&s| s <= offset) - 1;
        (line + 1, offset - self.line_starts[line] + 1)
    }

    pub fn text(&self, span: Span) -> &str {
        &self.source[span.start..span.end]
    }
}
