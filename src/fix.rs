//! Fixing rejected gram files (ADR-010).
//!
//! A fix is a set of edits to the source text. Fixes are proposed from a
//! diagnostic and the syntax tree, applied in rounds, and verified: the result
//! never has more errors than the input.

use crate::diagnostics::{code, Diagnostic};
use crate::names;
use crate::settings::LoadSettings;
use crate::syntax::{self, Syntax};
use tree_sitter::Node;

/// The most rounds of check, fix and re-check (ADR-010 section 2.3).
const MAX_ROUNDS: usize = 10;

/// Which way an undirected or bidirectional arrow points: `Right` keeps the
/// direction from the left node to the right node as written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Right,
    Left,
}

/// How to read an octal literal such as `010`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OctalAs {
    /// `010` is ten.
    Decimal,
    /// `010` is eight.
    Octal,
}

/// The decisions only the author can make. A correction that needs one is
/// applied only when it is given.
#[derive(Clone, Debug, Default)]
pub struct FixOptions {
    pub rel_type: Option<String>,
    pub direction: Option<Direction>,
    pub octal: Option<OctalAs>,
    /// Write a `// relate-gram:` comment above each line that still has an error
    /// (ADR-010 section 2.7). Earlier markers are always removed first.
    pub annotate: bool,
}

/// Replace the bytes `start..end` of the input with `replacement`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edit {
    pub start: usize,
    pub end: usize,
    pub replacement: String,
}

/// A correction for one diagnostic.
#[derive(Clone, Debug)]
pub struct Fix {
    pub code: &'static str,
    pub description: String,
    pub edits: Vec<Edit>,
}

/// The result of [`fix`].
#[derive(Clone, Debug)]
pub struct FixOutcome {
    /// The fixed text (equal to the input if nothing was applied).
    pub text: Vec<u8>,
    /// Every fix applied, in order.
    pub applied: Vec<Fix>,
    /// The diagnostics of the fixed text, with positions in it.
    pub remaining: Vec<Diagnostic>,
    /// How many rounds applied something.
    pub rounds: usize,
    /// How many earlier `// relate-gram:` markers were removed from the input.
    pub annotations_removed: usize,
    /// How many markers were written above lines that still have errors.
    pub annotations_added: usize,
}

impl FixOutcome {
    pub fn has_errors(&self) -> bool {
        self.remaining.iter().any(Diagnostic::is_error)
    }
}

/// The result would have more errors than the input, so nothing is offered.
#[derive(Clone, Debug, thiserror::Error)]
#[error("the fixes would leave {after} error(s) where the file had {before}; nothing was changed")]
pub struct FixFailure {
    pub before: usize,
    pub after: usize,
}

/// The option that would make a fix possible for `code`, as shown to the user.
pub fn needs(code: &str) -> Option<&'static str> {
    match code {
        code::REL_NO_TYPE => Some("--rel-type NAME"),
        code::REL_UNDIRECTED | code::REL_BIDIRECTIONAL => Some("--direction right|left"),
        code::VALUE_OCTAL => Some("--octal decimal|octal"),
        _ => None,
    }
}

fn errors_of(diags: &[Diagnostic]) -> usize {
    diags.iter().filter(|d| d.is_error()).count()
}

/// Apply `edits` to `text`. Edits are taken in order of position; one that
/// overlaps an earlier one is skipped. Returns the new text and how many edits
/// of each input index were applied.
fn apply(text: &[u8], mut edits: Vec<(usize, Edit)>) -> (Vec<u8>, Vec<usize>) {
    edits.sort_by_key(|(_, e)| (e.start, e.end));
    let mut kept: Vec<&(usize, Edit)> = Vec::new();
    let mut last_end = 0;
    for item in &edits {
        if item.1.start >= last_end && item.1.end <= text.len() {
            last_end = item.1.end;
            kept.push(item);
        }
    }
    let mut out = Vec::with_capacity(text.len());
    let mut at = 0;
    for (_, e) in &kept {
        out.extend_from_slice(&text[at..e.start]);
        out.extend_from_slice(e.replacement.as_bytes());
        at = e.end;
    }
    out.extend_from_slice(&text[at..]);
    (out, kept.iter().map(|(i, _)| *i).collect())
}

/// Fix `source` as far as the options allow (ADR-010 sections 2.3 and 2.4).
pub fn fix(
    source: &[u8],
    settings: &LoadSettings,
    options: &FixOptions,
) -> Result<FixOutcome, FixFailure> {
    let (stripped, annotations_removed) = strip_annotations(source);
    let before = errors_of(&crate::check(&stripped, settings).diagnostics);
    let mut text = stripped;
    let mut applied: Vec<Fix> = Vec::new();
    let mut rounds = 0;
    for _ in 0..MAX_ROUNDS {
        // A file with a syntax error is not fixed: its other errors are not known yet.
        let Ok(syn) = syntax::parse(&text, settings.max_diagnostics) else {
            break;
        };
        let plan = crate::plan::build(&syn, settings);
        let fixes: Vec<Fix> = plan
            .diagnostics
            .iter()
            .filter(|d| d.is_error())
            .filter_map(|d| propose(&syn, d, options))
            .collect();
        if fixes.is_empty() {
            break;
        }
        // One edit list across the fixes, remembering which fix each edit belongs to.
        let edits: Vec<(usize, Edit)> = fixes
            .iter()
            .enumerate()
            .flat_map(|(i, f)| f.edits.iter().cloned().map(move |e| (i, e)))
            .collect();
        let total: Vec<usize> = fixes.iter().map(|f| f.edits.len()).collect();
        let (new_text, used) = apply(&text, edits);
        let mut counts = vec![0usize; fixes.len()];
        for i in used {
            counts[i] += 1;
        }
        let done: Vec<Fix> = fixes
            .into_iter()
            .enumerate()
            .filter(|(i, _)| counts[*i] == total[*i])
            .map(|(_, f)| f)
            .collect();
        if done.is_empty() || new_text == text {
            break;
        }
        text = new_text;
        applied.extend(done);
        rounds += 1;
    }
    let mut remaining = crate::check(&text, settings).diagnostics;
    let after = errors_of(&remaining);
    if after > before {
        return Err(FixFailure { before, after });
    }
    let mut annotations_added = 0;
    if options.annotate {
        if let Some((annotated, n)) = annotate(&text, &remaining, settings) {
            // Markers are comments, so the errors are the same ones; keep them only if so.
            let again = crate::check(&annotated, settings).diagnostics;
            if errors_of(&again) == after {
                text = annotated;
                remaining = again;
                annotations_added = n;
            }
        }
    }
    Ok(FixOutcome {
        text,
        applied,
        remaining,
        rounds,
        annotations_removed,
        annotations_added,
    })
}

/// The prefix of a line comment that this tool writes and removes (ADR-010 section 2.7).
pub const MARKER: &str = "// relate-gram:";

/// Remove every line that is a `// relate-gram:` marker. Returns the text and how many lines were removed.
pub fn strip_annotations(text: &[u8]) -> (Vec<u8>, usize) {
    let bom: &[u8] = &[0xEF, 0xBB, 0xBF];
    let mut out = Vec::with_capacity(text.len());
    let mut removed = 0;
    for (i, line) in text.split_inclusive(|b| *b == b'\n').enumerate() {
        let body = if i == 0 {
            line.strip_prefix(bom).unwrap_or(line)
        } else {
            line
        };
        let trimmed = body
            .iter()
            .position(|b| !matches!(b, b' ' | b'\t'))
            .map(|p| &body[p..]);
        if trimmed.is_some_and(|t| t.starts_with(MARKER.as_bytes())) {
            if i == 0 && line.starts_with(bom) {
                out.extend_from_slice(bom);
            }
            removed += 1;
        } else {
            out.extend_from_slice(line);
        }
    }
    (out, removed)
}

/// Insert a marker line above the line of each error. A line inside a multi-line
/// string is never split: the marker goes above the line where the string starts.
fn annotate(
    text: &[u8],
    diags: &[Diagnostic],
    settings: &LoadSettings,
) -> Option<(Vec<u8>, usize)> {
    let errors: Vec<&Diagnostic> = diags
        .iter()
        .filter(|d| d.is_error() && d.span.is_some())
        .collect();
    if errors.is_empty() {
        return None;
    }
    let syn = syntax::parse(text, settings.max_diagnostics).ok();
    let bom_len = if text.starts_with(&[0xEF, 0xBB, 0xBF]) {
        3
    } else {
        0
    };
    let eol = if text.windows(2).any(|w| w == b"\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    // The byte ranges of strings that span lines: a marker line must not start inside one.
    let mut strings: Vec<(usize, usize)> = Vec::new();
    if let Some(syn) = &syn {
        fn walk(n: Node<'_>, base: usize, out: &mut Vec<(usize, usize)>) {
            if matches!(n.kind(), "string_literal" | "tagged_string")
                && n.start_position().row != n.end_position().row
            {
                out.push((n.start_byte() + base, n.end_byte() + base));
            }
            let mut c = n.walk();
            for child in n.children(&mut c) {
                walk(child, base, out);
            }
        }
        walk(syn.root(), syn.base(), &mut strings);
    }
    let line_start_of = |at: usize| {
        text[..at]
            .iter()
            .rposition(|b| *b == b'\n')
            .map_or(0, |p| p + 1)
    };
    let mut inserts: Vec<(usize, usize, String)> = Vec::new();
    for (order, d) in errors.iter().enumerate() {
        let span = d.span?;
        let mut line_start = line_start_of(span.start.min(text.len()));
        // Move up while the line starts inside a multi-line string.
        while let Some((start, _)) = strings
            .iter()
            .find(|(start, end)| *start < line_start && line_start < *end)
        {
            line_start = line_start_of(*start);
        }
        if line_start < bom_len {
            line_start = bom_len;
        }
        let indent: String = text[line_start..]
            .iter()
            .take_while(|b| matches!(b, b' ' | b'\t'))
            .map(|b| *b as char)
            .collect();
        let message = d.message.replace(['\r', '\n'], " ");
        inserts.push((
            line_start,
            order,
            format!("{indent}{MARKER} error[{}]: {message}{eol}", d.code),
        ));
    }
    inserts.sort_by_key(|(at, order, _)| (*at, *order));
    let mut out = Vec::with_capacity(text.len() + inserts.len() * 80);
    let mut pos = 0;
    for (at, _, line) in &inserts {
        out.extend_from_slice(&text[pos..*at]);
        out.extend_from_slice(line.as_bytes());
        pos = *at;
    }
    out.extend_from_slice(&text[pos..]);
    Some((out, inserts.len()))
}

/// The arrow node that contains `node`, if any.
fn arrow_of(node: Node<'_>) -> Option<Node<'_>> {
    let mut n = Some(node);
    while let Some(x) = n {
        if x.kind().ends_with("_arrow") {
            return Some(x);
        }
        n = x.parent();
    }
    None
}

fn ancestor<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    let mut n = Some(node);
    while let Some(x) = n {
        if x.kind() == kind {
            return Some(x);
        }
        n = x.parent();
    }
    None
}

/// The byte offsets, in the stripped text, of the `[` and `]` of an arrow.
fn brackets(arrow: Node<'_>) -> Option<(usize, usize)> {
    let mut open = None;
    let mut close = None;
    let mut cursor = arrow.walk();
    for child in arrow.children(&mut cursor) {
        match child.kind() {
            "[" if open.is_none() => open = Some(child.start_byte()),
            "]" => close = Some(child.end_byte()),
            _ => {}
        }
    }
    Some((open?, close?))
}

/// An integer literal as the text of the same value written as a decimal, if a
/// 64-bit float holds it exactly.
fn as_decimal(text: &str) -> Option<String> {
    let v: i64 = match text.strip_prefix("0x") {
        Some(hex) => i64::from_str_radix(hex, 16).ok()?,
        None => text.parse().ok()?,
    };
    const EXACT: i64 = 1 << 53;
    if !(-EXACT..=EXACT).contains(&v) {
        return None;
    }
    let mut s = format!("{}", v as f64);
    if !s.contains('.') {
        s.push_str(".0");
    }
    Some(s)
}

/// Propose a fix for `d`, or `None` if it has none (or needs an option that was not given).
pub fn propose(syn: &Syntax, d: &Diagnostic, options: &FixOptions) -> Option<Fix> {
    let span = d.span?;
    let node = syn.node_at(&span)?;
    let base = syn.base();
    let abs = |n: usize| n + base;
    let edit = |start: usize, end: usize, replacement: String| Edit {
        start,
        end,
        replacement,
    };

    match d.code {
        code::REL_NO_TYPE => {
            let ty = names::name(options.rel_type.as_deref()?);
            let arrow = arrow_of(node)?;
            let text = syn.text(arrow);
            let edits = match brackets(arrow) {
                None => {
                    let new = match arrow.kind() {
                        "right_arrow" => format!("-[:{ty}]->"),
                        "left_arrow" => format!("<-[:{ty}]-"),
                        _ => return None,
                    };
                    vec![edit(abs(arrow.start_byte()), abs(arrow.end_byte()), new)]
                }
                Some((open, _)) => match arrow.child_by_field_name("identifier") {
                    Some(id) => vec![edit(
                        abs(id.end_byte()),
                        abs(id.end_byte()),
                        format!(":{ty}"),
                    )],
                    None => {
                        let next = syn
                            .source()
                            .as_bytes()
                            .get(open + 1)
                            .copied()
                            .unwrap_or(b']');
                        let pad = if next.is_ascii_whitespace() || next == b']' {
                            ""
                        } else {
                            " "
                        };
                        vec![edit(abs(open + 1), abs(open + 1), format!(":{ty}{pad}"))]
                    }
                },
            };
            let _ = text;
            Some(Fix {
                code: d.code,
                description: format!(
                    "insert the relationship type {}",
                    options.rel_type.as_deref()?
                ),
                edits,
            })
        }
        code::REL_UNDIRECTED | code::REL_BIDIRECTIONAL => {
            let dir = options.direction?;
            let arrow = arrow_of(node)?;
            let (start, end) = (abs(arrow.start_byte()), abs(arrow.end_byte()));
            let text = syn.text(arrow);
            let (new, what) = if d.code == code::REL_UNDIRECTED {
                match dir {
                    Direction::Right => (format!("{text}>"), "point the arrow right"),
                    Direction::Left => (format!("<{text}"), "point the arrow left"),
                }
            } else {
                match dir {
                    Direction::Right => (text.get(1..)?.to_string(), "point the arrow right"),
                    Direction::Left => (
                        text.get(..text.len() - 1)?.to_string(),
                        "point the arrow left",
                    ),
                }
            };
            Some(Fix {
                code: d.code,
                description: what.to_string(),
                edits: vec![edit(start, end, new)],
            })
        }
        code::REL_ARROW_FAMILY => {
            let arrow = arrow_of(node)?;
            let text = syn.text(arrow);
            let thin = |s: &str| s.replace(['=', '~'], "-");
            let new = match brackets(arrow) {
                None => thin(text),
                Some((open, close)) => {
                    let rel_open = open - arrow.start_byte();
                    let rel_close = close - arrow.start_byte();
                    format!(
                        "{}{}{}",
                        thin(&text[..rel_open]),
                        &text[rel_open..rel_close],
                        thin(&text[rel_close..])
                    )
                }
            };
            Some(Fix {
                code: d.code,
                description: "write a thin arrow".to_string(),
                edits: vec![edit(abs(arrow.start_byte()), abs(arrow.end_byte()), new)],
            })
        }
        code::CONSTRUCT_DOUBLE_COLON => Some(Fix {
            code: d.code,
            description: "write a single colon".to_string(),
            edits: vec![edit(span.start, span.end, ":".to_string())],
        }),
        code::VALUE_OCTAL => {
            let reading = options.octal?;
            let text = syn.text(node);
            let v = match reading {
                OctalAs::Decimal => text.parse::<i64>().ok()?,
                OctalAs::Octal => i64::from_str_radix(text, 8).ok()?,
            };
            Some(Fix {
                code: d.code,
                description: format!("write the octal literal {text} as {v}"),
                edits: vec![edit(span.start, span.end, v.to_string())],
            })
        }
        code::VALUE_ARRAY_MIXED => {
            let array = ancestor(node, "array")?;
            let mut edits = Vec::new();
            let mut cursor = array.walk();
            let mut saw_decimal = false;
            for el in array.named_children(&mut cursor) {
                match el.kind() {
                    "comment" | "doc_comment" => {}
                    "decimal" => saw_decimal = true,
                    "integer" | "hexadecimal" => {
                        let new = as_decimal(syn.text(el))?;
                        edits.push(edit(abs(el.start_byte()), abs(el.end_byte()), new));
                    }
                    _ => return None,
                }
            }
            if !saw_decimal || edits.is_empty() {
                return None;
            }
            Some(Fix {
                code: d.code,
                description: "write the integers of the array as decimals".to_string(),
                edits,
            })
        }
        code::VALUE_TAG => {
            let tagged = ancestor(node, "tagged_string")?;
            let tag = tagged.child_by_field_name("tag")?;
            Some(Fix {
                code: d.code,
                description: format!("remove the tag {}", syn.text(tag)),
                edits: vec![edit(
                    abs(tag.start_byte()),
                    abs(tag.end_byte()),
                    String::new(),
                )],
            })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(start: usize, end: usize, r: &str) -> Edit {
        Edit {
            start,
            end,
            replacement: r.to_string(),
        }
    }

    #[test]
    fn edits_apply_in_position_order_and_overlaps_are_skipped() {
        let (out, used) = apply(
            b"0123456789",
            vec![(1, e(5, 7, "X")), (0, e(1, 3, "ab")), (2, e(2, 4, "!"))],
        );
        // edit 2 overlaps edit 0 (1..3) and is skipped
        assert_eq!(out, b"0ab34X789".to_vec());
        assert_eq!(used, vec![0, 1]);
    }

    #[test]
    fn insertions_at_one_position_are_kept() {
        let (out, _) = apply(b"ab", vec![(0, e(1, 1, "X"))]);
        assert_eq!(out, b"aXb".to_vec());
    }

    #[test]
    fn integers_become_decimals_only_when_exact() {
        assert_eq!(as_decimal("1").as_deref(), Some("1.0"));
        assert_eq!(as_decimal("-3").as_deref(), Some("-3.0"));
        assert_eq!(as_decimal("0x1F").as_deref(), Some("31.0"));
        assert_eq!(as_decimal("9007199254740993"), None);
    }

    #[test]
    fn each_fixable_code_names_the_option_it_needs_or_none() {
        assert_eq!(needs("rel.no-type"), Some("--rel-type NAME"));
        assert_eq!(needs("rel.arrow-family"), None);
    }
}
