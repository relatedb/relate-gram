//! Parsing gram text into the tree-sitter-gram syntax tree (ADR-007).
//!
//! The input is read as bytes. Input that is not valid UTF-8 is an error at
//! the first bad byte, and a leading byte order mark is skipped. If the tree
//! has any ERROR or MISSING node, every one is reported and no tree is
//! returned, so nothing is read from a malformed file.

use crate::diagnostics::{code, Diagnostic, LineIndex, Span};
use tree_sitter::{Node, Parser, Tree};

const BOM: &[u8] = &[0xEF, 0xBB, 0xBF];

/// A parsed, error-free gram file.
pub struct Syntax {
    text: String,
    tree: Tree,
    index: LineIndex,
    /// The length of a skipped byte order mark: spans and edit ranges count it.
    base: usize,
}

/// The diagnostics from a failed parse: up to the cap, with the total found.
#[derive(Debug)]
pub struct SyntaxFailure {
    pub diagnostics: Vec<Diagnostic>,
    pub total: usize,
}

impl Syntax {
    pub fn root(&self) -> Node<'_> {
        self.tree.root_node()
    }

    /// The source text of a node.
    pub fn text(&self, node: Node<'_>) -> &str {
        &self.text[node.byte_range()]
    }

    pub fn span(&self, node: Node<'_>) -> Span {
        self.index.span(node.start_byte(), node.end_byte())
    }

    pub fn span_of_range(&self, start: usize, end: usize) -> Span {
        self.index.span(start, end)
    }

    /// The length of the byte order mark that was skipped before parsing, or 0.
    pub fn base(&self) -> usize {
        self.base
    }

    /// The smallest node that covers `span`.
    pub fn node_at(&self, span: &Span) -> Option<Node<'_>> {
        let start = span.start.checked_sub(self.base)?;
        let end = span.end.checked_sub(self.base)?;
        self.tree.root_node().descendant_for_byte_range(start, end)
    }

    /// The whole text after any byte order mark.
    pub fn source(&self) -> &str {
        &self.text
    }
}

/// Parse `input`. On failure returns up to `max_diagnostics` diagnostics
/// in source order and the total number found.
pub fn parse(input: &[u8], max_diagnostics: usize) -> Result<Syntax, SyntaxFailure> {
    let (body, base) = match input.strip_prefix(BOM) {
        Some(rest) => (rest, BOM.len()),
        None => (input, 0),
    };
    let index = LineIndex::new(body, base);
    let text = match std::str::from_utf8(body) {
        Ok(t) => t.to_string(),
        Err(e) => {
            let at = e.valid_up_to();
            let span = index.span(at, at + e.error_len().unwrap_or(1));
            return Err(SyntaxFailure {
                diagnostics: vec![Diagnostic::error(
                    code::INPUT_ENCODING,
                    span,
                    "the input is not valid UTF-8",
                )],
                total: 1,
            });
        }
    };

    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_gram::LANGUAGE.into())
        .expect("the tree-sitter-gram grammar matches the tree-sitter version");
    let tree = parser
        .parse(&text, None)
        .expect("parsing without a timeout always returns a tree");

    if tree.root_node().has_error() {
        let mut found = Vec::new();
        collect_errors(tree.root_node(), &index, &mut found);
        found.sort_by_key(|d| d.span.map(|s| s.start));
        let total = found.len();
        found.truncate(max_diagnostics);
        return Err(SyntaxFailure {
            diagnostics: found,
            total,
        });
    }
    Ok(Syntax {
        text,
        tree,
        index,
        base,
    })
}

fn collect_errors(node: Node<'_>, index: &LineIndex, out: &mut Vec<Diagnostic>) {
    if node.is_missing() {
        out.push(Diagnostic::error(
            code::PARSE_MISSING,
            index.span(node.start_byte(), node.end_byte()),
            format!("expected {}", node.kind()),
        ));
        return;
    }
    if node.is_error() {
        out.push(Diagnostic::error(
            code::PARSE_ERROR,
            index.span(node.start_byte(), node.end_byte()),
            "syntax error",
        ));
        return;
    }
    if !node.has_error() {
        return;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_errors(child, index, out);
    }
}
