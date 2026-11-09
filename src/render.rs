//! Rendering a diagnostic for a person: the message, where it is, the source
//! line, and a marker under the text at fault.
//!
//! ```text
//! error[rel.no-type]: a relationship needs a type; write one, for example `-[:KNOWS]->`
//!  --> people.gram:2:6
//!   |
//! 2 | (b:N)-->(c:N)
//!   |      ^^^
//! ```

use crate::diagnostics::{Diagnostic, Severity, Span};

const BOM: &[u8] = &[0xEF, 0xBB, 0xBF];

/// The bytes of line `n` (1-based) of `source`, without its line ending or a byte order mark.
fn line_bytes(source: &[u8], n: usize) -> &[u8] {
    let body = source.strip_prefix(BOM).unwrap_or(source);
    let line = body
        .split(|b| *b == b'\n')
        .nth(n.saturating_sub(1))
        .unwrap_or(&[]);
    line.strip_suffix(b"\r").unwrap_or(line)
}

/// `count` columns of padding that line up with the source line, copying its tabs.
fn pad_like(prefix: &str) -> String {
    prefix
        .chars()
        .map(|c| if c == '\t' { '\t' } else { ' ' })
        .collect()
}

fn snippet(out: &mut String, source: &[u8], span: Span, width: usize, marker: char, label: &str) {
    let line = line_bytes(source, span.line);
    let start = span.column.saturating_sub(1).min(line.len());
    let end = (start + span.end.saturating_sub(span.start)).min(line.len());
    let prefix = String::from_utf8_lossy(&line[..start]).into_owned();
    let marked = String::from_utf8_lossy(&line[start..end])
        .chars()
        .count()
        .max(1);
    let gutter = " ".repeat(width);
    out.push_str(&format!("{gutter} |\n"));
    out.push_str(&format!(
        "{:>width$} | {}\n",
        span.line,
        String::from_utf8_lossy(line)
    ));
    let mut marks = marker.to_string().repeat(marked);
    if !label.is_empty() {
        marks.push(' ');
        marks.push_str(label);
    }
    out.push_str(&format!("{gutter} | {}{marks}\n", pad_like(&prefix)));
}

/// Render `d` against `source`, naming the file `name`. The text ends with a newline.
pub fn render(d: &Diagnostic, source: &[u8], name: &str) -> String {
    let severity = match d.severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
    };
    let mut out = format!("{severity}[{}]: {}\n", d.code, d.message);
    let Some(span) = d.span else {
        return out;
    };
    let width = d
        .related
        .iter()
        .map(|(_, s)| s.line)
        .chain([span.line])
        .max()
        .unwrap_or(1)
        .to_string()
        .len();
    out.push_str(&format!(
        "{}--> {name}:{}:{}\n",
        " ".repeat(width),
        span.line,
        span.column
    ));
    snippet(&mut out, source, span, width, '^', "");
    for (label, s) in &d.related {
        snippet(&mut out, source, *s, width, '-', label);
    }
    out
}
