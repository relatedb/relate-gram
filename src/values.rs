//! Property values: gram value nodes to Cypher values on load (ADR-006
//! sections 2.1 to 2.3), and Cypher values to gram text on export.

use crate::diagnostics::{code, Diagnostic};
use crate::settings::TagPolicy;
use crate::syntax::Syntax;
use tree_sitter::Node;

/// A property value the loader can store: a Cypher scalar or a list of one scalar kind.
#[derive(Clone, Debug, PartialEq)]
pub enum PropValue {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    List(Vec<PropValue>),
}

/// Resolve the escapes of a quoted string or name. `quote` is the style's own
/// quote character, which may also be escaped.
pub(crate) fn unescape(content: &str, quote: char) -> String {
    let mut out = String::with_capacity(content.len());
    let mut chars = content.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('b') => out.push('\u{8}'),
            Some('f') => out.push('\u{c}'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some('/') => out.push('/'),
            Some('\\') => out.push('\\'),
            Some(q) if q == quote => out.push(q),
            // The grammar admits no other escape, so this is unreachable for a clean tree.
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}

/// The text of a string-like node's `content` child (empty if absent).
fn content<'a>(syn: &'a Syntax, node: Node<'_>) -> &'a str {
    node.child_by_field_name("content")
        .map(|n| syn.text(n))
        .unwrap_or("")
}

/// The text of a `string_literal` node with escapes resolved by its quoting style.
pub(crate) fn string_value(syn: &Syntax, node: Node<'_>) -> String {
    let text = syn.text(node);
    let raw = content(syn, node);
    if text.starts_with("```") {
        raw.to_string()
    } else {
        let quote = text.chars().next().unwrap_or('"');
        unescape(raw, quote)
    }
}

/// The name written by a key or identifier node: a symbol, an integer, or a
/// quoted name.
pub(crate) fn name_text(syn: &Syntax, node: Node<'_>) -> String {
    match node.kind() {
        "quoted_name" => unescape(content(syn, node), '`'),
        "double_quoted_name" => unescape(content(syn, node), '"'),
        _ => syn.text(node).to_string(),
    }
}

/// Convert one value node. Pushes a diagnostic and returns `None` for a value
/// that cannot be stored.
pub(crate) fn convert_value(
    syn: &Syntax,
    node: Node<'_>,
    tags: TagPolicy,
    diags: &mut Vec<Diagnostic>,
) -> Option<PropValue> {
    let text = syn.text(node);
    match node.kind() {
        "integer" => match text.parse::<i64>() {
            Ok(i) => Some(PropValue::Int(i)),
            Err(_) => unsupported(syn, node, diags),
        },
        "hexadecimal" => match i64::from_str_radix(&text[2..], 16) {
            Ok(i) => Some(PropValue::Int(i)),
            Err(_) => unsupported(syn, node, diags),
        },
        "decimal" => match text.parse::<f64>() {
            Ok(f) => Some(PropValue::Float(f)),
            Err(_) => unsupported(syn, node, diags),
        },
        "boolean_literal" => Some(PropValue::Bool(text == "true")),
        "symbol" => Some(PropValue::Str(text.to_string())),
        "string_literal" => Some(PropValue::Str(string_value(syn, node))),
        "tagged_string" => {
            let tag = node
                .child_by_field_name("tag")
                .map(|t| syn.text(t))
                .unwrap_or("");
            let _ = tags; // the error policy arrives with the rejection rules
            diags.push(Diagnostic::warning(
                code::VALUE_TAG_DROPPED,
                syn.span(node),
                format!("the tag `{tag}` was dropped; the content was loaded as a string"),
            ));
            let raw = content(syn, node);
            if text.starts_with("```") {
                Some(PropValue::Str(raw.to_string()))
            } else {
                Some(PropValue::Str(unescape(raw, '`')))
            }
        }
        "array" => {
            let mut items = Vec::new();
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                if child.kind() == "comment" || child.kind() == "doc_comment" {
                    continue;
                }
                items.push(convert_value(syn, child, tags, diags)?);
            }
            Some(PropValue::List(items))
        }
        _ => unsupported(syn, node, diags),
    }
}

fn unsupported(syn: &Syntax, node: Node<'_>, diags: &mut Vec<Diagnostic>) -> Option<PropValue> {
    diags.push(Diagnostic::error(
        code::CONSTRUCT_UNSUPPORTED,
        syn.span(node),
        format!(
            "the value `{}` ({}) is not supported",
            syn.text(node),
            node.kind()
        ),
    ));
    None
}

/// Convert a record node to its ordered key and value pairs.
pub(crate) fn convert_record(
    syn: &Syntax,
    record: Node<'_>,
    tags: TagPolicy,
    diags: &mut Vec<Diagnostic>,
) -> Vec<(String, PropValue)> {
    let mut out = Vec::new();
    let mut cursor = record.walk();
    for prop in record.named_children(&mut cursor) {
        if prop.kind() != "record_property" {
            continue;
        }
        let (Some(key), Some(value)) = (
            prop.child_by_field_name("key"),
            prop.child_by_field_name("value"),
        ) else {
            continue;
        };
        let key = name_text(syn, key);
        if let Some(v) = convert_value(syn, value, tags, diags) {
            out.push((key, v));
        }
    }
    out
}
