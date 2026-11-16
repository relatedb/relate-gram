//! SPIKE-007 prototype: a typed, position-carrying view of a gram file.
//!
//! The tree keeps what the grammar says (node and relationship forms, arrow family and
//! direction, `:` versus `::`, repeated keys, doc comments, annotations) and lowers the leaves to
//! `gram_value::Value`s. Nothing is rejected here: a value that gram-value can hold, such as an
//! integer beyond 64 bits, is held. `Diagnostic`s report what is wrong with gram itself (syntax,
//! repeated keys, bad tag content, a defined element defined again); a consumer layers its own
//! policy on the tree. `project` gives the `Pattern<Subject>` view with a source map.

mod lower;
mod project;
mod syntax;

pub use gram_value::Value;
pub use project::{project, Pattern, PatternSubject};
pub use syntax::*;

/// Parse `source` into a `Document`. Never fails: problems are in `Document::diagnostics`.
pub fn parse(source: &str) -> Document {
    lower::document(source)
}
