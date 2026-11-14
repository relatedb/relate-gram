//! Property values: gram value nodes to Cypher values on load (ADR-006
//! sections 2.1 to 2.3), and Cypher values to gram text on export.

use crate::diagnostics::{code, Diagnostic};
use crate::settings::TagPolicy;
use crate::syntax::Syntax;
use crate::typed::{Tag, TypedValue};
use tree_sitter::Node;

/// A property value the loader can store: a Cypher scalar, a typed value (ADR-011), or a
/// list of one kind.
#[derive(Clone, Debug, PartialEq)]
pub enum PropValue {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    /// A tagged string with a recognized tag, validated; the database builds the value.
    Typed(TypedValue),
    List(Vec<PropValue>),
}

impl PropValue {
    /// Whether the value is a typed value or a list of them. The loader writes these with
    /// constructor statements after the other properties (ADR-011 section 2.7).
    pub fn is_typed(&self) -> bool {
        match self {
            PropValue::Typed(_) => true,
            PropValue::List(items) => items.first().is_some_and(PropValue::is_typed),
            _ => false,
        }
    }
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

/// Which scalar kind a value is, for the homogeneous-array rule (ADR-006 section 2.2).
fn kind_name(v: &PropValue) -> &'static str {
    match v {
        PropValue::Int(_) => "integer",
        PropValue::Float(_) => "decimal",
        PropValue::Bool(_) => "boolean",
        PropValue::Str(_) => "string",
        PropValue::Typed(t) => t.tag.name(),
        PropValue::List(_) => "list",
    }
}

fn reject(
    syn: &Syntax,
    node: Node<'_>,
    diag_code: &'static str,
    message: String,
    diags: &mut Vec<Diagnostic>,
) -> Option<PropValue> {
    diags.push(Diagnostic::error(diag_code, syn.span(node), message));
    None
}

/// Convert one value node. Pushes a diagnostic and returns `None` for a value
/// that cannot be stored (ADR-006 sections 2.1 to 2.3).
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
            Err(_) => reject(
                syn,
                node,
                code::VALUE_INTEGER_RANGE,
                format!("the integer `{}` does not fit in 64 bits", abbreviate(text)),
                diags,
            ),
        },
        "hexadecimal" => match i64::from_str_radix(&text[2..], 16) {
            Ok(i) => Some(PropValue::Int(i)),
            Err(_) => reject(
                syn,
                node,
                code::VALUE_INTEGER_RANGE,
                format!("the integer `{}` does not fit in 64 bits", abbreviate(text)),
                diags,
            ),
        },
        "decimal" => {
            let f: f64 = text.parse().unwrap_or(f64::INFINITY);
            let nonzero_digits = text.chars().any(|c| ('1'..='9').contains(&c));
            if !f.is_finite() {
                reject(
                    syn,
                    node,
                    code::VALUE_DECIMAL,
                    format!("the decimal `{}` is not finite as a 64-bit float", abbreviate(text)),
                    diags,
                )
            } else if f == 0.0 && nonzero_digits {
                reject(
                    syn,
                    node,
                    code::VALUE_DECIMAL,
                    format!("the decimal `{}` rounds to zero as a 64-bit float", abbreviate(text)),
                    diags,
                )
            } else {
                Some(PropValue::Float(f))
            }
        }
        "boolean_literal" => Some(PropValue::Bool(text == "true")),
        "symbol" => Some(PropValue::Str(text.to_string())),
        "string_literal" => Some(PropValue::Str(string_value(syn, node))),
        "tagged_string" => {
            let tag = node.child_by_field_name("tag").map(|t| syn.text(t)).unwrap_or("");
            let raw = content(syn, node);
            let body = if text.starts_with("```") {
                raw.to_string()
            } else {
                unescape(raw, '`')
            };
            // A recognized tag gives a typed value, or an error: never a plain string
            // (ADR-011 sections 2.2 and 2.3).
            if let Some(recognized) = Tag::from_name(tag) {
                return match recognized.validate(&body) {
                    Ok(content) => Some(PropValue::Typed(TypedValue {
                        tag: recognized,
                        content,
                        span: syn.span(node),
                    })),
                    Err(fault) => reject(
                        syn,
                        node,
                        code::VALUE_TAG_CONTENT,
                        format!("the content of the `{tag}` tag is not valid: {fault}"),
                        diags,
                    ),
                };
            }
            let diag = match tags {
                TagPolicy::Warn => Diagnostic::warning(
                    code::VALUE_TAG_DROPPED,
                    syn.span(node),
                    format!("the tag `{tag}` was dropped; the content was loaded as a string"),
                ),
                TagPolicy::Error => Diagnostic::error(
                    code::VALUE_TAG,
                    syn.span(node),
                    format!("the tag `{tag}` is not supported; remove it to load the content as a string"),
                ),
            };
            diags.push(diag);
            Some(PropValue::Str(body))
        }
        "array" => {
            let mut items: Vec<(PropValue, Node<'_>)> = Vec::new();
            let mut ok = true;
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                if child.kind() == "comment" || child.kind() == "doc_comment" {
                    continue;
                }
                match convert_value(syn, child, tags, diags) {
                    Some(v) => items.push((v, child)),
                    None => ok = false,
                }
            }
            if !ok {
                return None;
            }
            if let Some((first, _)) = items.first() {
                let want = kind_name(first);
                if let Some((v, at)) = items.iter().find(|(v, _)| kind_name(v) != want) {
                    return reject(
                        syn,
                        *at,
                        code::VALUE_ARRAY_MIXED,
                        format!(
                            "an array holds one kind of value; found a {} after {} value(s)",
                            kind_name(v),
                            want
                        ),
                        diags,
                    );
                }
            }
            Some(PropValue::List(items.into_iter().map(|(v, _)| v).collect()))
        }
        "range" => reject(
            syn,
            node,
            code::VALUE_RANGE,
            format!("the range `{text}` has no Cypher type; write the bounds as separate properties"),
            diags,
        ),
        "measurement" => reject(
            syn,
            node,
            code::VALUE_MEASUREMENT,
            format!("the measurement `{text}` has no Cypher type; write it as a number and a unit property"),
            diags,
        ),
        "map" => reject(
            syn,
            node,
            code::VALUE_MAP,
            "a nested map has no Cypher type; write its entries as separate properties".to_string(),
            diags,
        ),
        "octal" => reject(
            syn,
            node,
            code::VALUE_OCTAL,
            format!("the octal literal `{text}` is ambiguous; write the value in decimal or hexadecimal"),
            diags,
        ),
        other => reject(
            syn,
            node,
            code::CONSTRUCT_UNSUPPORTED,
            format!("the value `{}` ({other}) is not supported", abbreviate(text)),
            diags,
        ),
    }
}

/// Shorten long literal text for messages.
fn abbreviate(text: &str) -> String {
    const MAX: usize = 24;
    if text.chars().count() <= MAX {
        text.to_string()
    } else {
        let head: String = text.chars().take(MAX).collect();
        format!("{head}...")
    }
}

/// Convert a record node to its ordered key and value pairs, checking each key
/// (ADR-006 section 2.4, ADR-001 section 2.8). Returns the pairs that are valid;
/// problems are pushed to `diags`.
pub(crate) fn convert_record(
    syn: &Syntax,
    record: Node<'_>,
    tags: TagPolicy,
    diags: &mut Vec<Diagnostic>,
) -> Vec<(String, PropValue)> {
    let mut out: Vec<(String, PropValue)> = Vec::new();
    let mut seen: Vec<(String, crate::diagnostics::Span)> = Vec::new();
    let mut cursor = record.walk();
    for prop in record.named_children(&mut cursor) {
        if prop.kind() != "record_property" {
            continue;
        }
        let (Some(key_node), Some(value)) = (
            prop.child_by_field_name("key"),
            prop.child_by_field_name("value"),
        ) else {
            continue;
        };
        let key = name_text(syn, key_node);
        let key_span = syn.span(key_node);
        let mut key_ok = true;
        if key.is_empty() {
            diags.push(Diagnostic::error(
                code::KEY_EMPTY,
                key_span,
                "a property key cannot be empty",
            ));
            key_ok = false;
        } else if key.starts_with('@') {
            diags.push(Diagnostic::error(
                code::KEY_RESERVED,
                key_span,
                format!("the key `{key}` is reserved: keys that start with `@` are written by the loader"),
            ));
            key_ok = false;
        } else if let Some((_, first)) = seen.iter().find(|(k, _)| *k == key) {
            diags.push(
                Diagnostic::error(
                    code::KEY_DUPLICATE,
                    key_span,
                    format!("the key `{key}` is repeated in one record"),
                )
                .with_related("first written here", *first),
            );
            key_ok = false;
        }
        seen.push((key.clone(), key_span));
        let converted = convert_value(syn, value, tags, diags);
        if let (true, Some(v)) = (key_ok, converted) {
            out.push((key, v));
        }
    }
    out
}
