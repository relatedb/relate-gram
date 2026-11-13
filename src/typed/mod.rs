//! Typed values: the eight tags of ADR-011 section 2.2 and the content forms they accept.
//!
//! Everything here works on text, with no database, so `check` can find a bad value
//! without one. The loader turns a validated [`TypedValue`] into a Relate value with a
//! Cypher constructor (ADR-011 section 2.7), and the exporter writes one back.

pub mod point;
pub mod temporal;
pub mod uuid;

use crate::diagnostics::Span;

/// The tag of a tagged string that gives it a type. Tags are matched exactly, in lower case.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Tag {
    Date,
    Time,
    LocalTime,
    DateTime,
    LocalDateTime,
    Duration,
    Point,
    Uuid,
}

impl Tag {
    /// Every recognized tag.
    pub const ALL: [Tag; 8] = [
        Tag::Date,
        Tag::Time,
        Tag::LocalTime,
        Tag::DateTime,
        Tag::LocalDateTime,
        Tag::Duration,
        Tag::Point,
        Tag::Uuid,
    ];

    /// The tag a name spells, or `None` for an unrecognized name.
    pub fn from_name(name: &str) -> Option<Tag> {
        Tag::ALL.into_iter().find(|t| t.name() == name)
    }

    /// The spelling of the tag in gram text.
    pub fn name(self) -> &'static str {
        match self {
            Tag::Date => "date",
            Tag::Time => "time",
            Tag::LocalTime => "localtime",
            Tag::DateTime => "datetime",
            Tag::LocalDateTime => "localdatetime",
            Tag::Duration => "duration",
            Tag::Point => "point",
            Tag::Uuid => "uuid",
        }
    }

    /// Check `text` against the content form of this tag (ADR-011 section 2.2).
    /// The error is a short description of the fault, for a `value.tag-content` message.
    pub fn validate(self, text: &str) -> Result<TypedContent, String> {
        match self {
            Tag::Date => temporal::date(text),
            Tag::Time => temporal::time(text),
            Tag::LocalTime => temporal::local_time(text),
            Tag::DateTime => temporal::date_time(text),
            Tag::LocalDateTime => temporal::local_date_time(text),
            Tag::Duration => temporal::duration(text),
            Tag::Point => point::point(text).map(TypedContent::Point),
            Tag::Uuid => uuid::uuid(text),
        }
    }
}

/// The parsed numbers of a point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointContent {
    pub srid: i64,
    pub x: f64,
    pub y: f64,
    pub z: Option<f64>,
}

/// What a validated tagged string holds.
#[derive(Clone, Debug, PartialEq)]
pub enum TypedContent {
    /// The text to pass to the constructor, in its accepted form.
    Text(String),
    Point(PointContent),
}

/// A validated tagged string: the tag, the content, and where it was written.
#[derive(Clone, Debug, PartialEq)]
pub struct TypedValue {
    pub tag: Tag,
    pub content: TypedContent,
    pub span: Span,
}
