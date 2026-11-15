//! CST to typed syntax. One pass, no early exit: whatever the tree holds is lowered, and what
//! is wrong is reported.
use crate::syntax::*;
use gram_value::{tags::interpret_tag, Diagnostic as ValueDiagnostic, PathSegment, Value};
use tree_sitter::{Node, Parser};

struct Ctx<'a> {
    src: &'a str,
    diags: Vec<Diagnostic>,
    docs: Vec<DocComment>,
}

pub(crate) fn document(source: &str) -> Document {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_gram::LANGUAGE.into())
        .expect("grammar matches tree-sitter");
    let tree = parser.parse(source, None).expect("a tree");
    let mut line_starts = vec![0];
    for (i, b) in source.bytes().enumerate() {
        if b == b'\n' {
            line_starts.push(i + 1);
        }
    }
    let mut cx = Ctx {
        src: source,
        diags: Vec::new(),
        docs: Vec::new(),
    };
    let root = tree.root_node();
    if root.has_error() {
        syntax_errors(root, &mut cx);
    }
    let mut record = None;
    let mut elements = Vec::new();
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        match child.kind() {
            "record" => record = Some(cx.record(child)),
            "comment" => {}
            "doc_comment" => cx.doc(child),
            _ => {
                if let Some(e) = cx.element(child) {
                    elements.push(e);
                }
            }
        }
    }
    let mut diagnostics = cx.diags;
    diagnostics.sort_by_key(|d| d.span.start);
    Document {
        source: source.to_string(),
        record,
        elements,
        doc_comments: cx.docs,
        diagnostics,
        line_starts,
    }
}

fn span(n: Node<'_>) -> Span {
    Span {
        start: n.start_byte(),
        end: n.end_byte(),
    }
}

fn syntax_errors(node: Node<'_>, cx: &mut Ctx<'_>) {
    if node.is_missing() {
        cx.error("missing-node", span(node), format!("expected {}", node.kind()));
        return;
    }
    if node.is_error() {
        cx.error("syntax-error", span(node), "syntax error".to_string());
        return;
    }
    if !node.has_error() {
        return;
    }
    let mut cursor = node.walk();
    for c in node.children(&mut cursor) {
        syntax_errors(c, cx);
    }
}

fn unescape(content: &str, quote: char) -> String {
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
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}

impl<'a> Ctx<'a> {
    fn text(&self, n: Node<'_>) -> &'a str {
        &self.src[n.byte_range()]
    }

    fn error(&mut self, code: &'static str, span: Span, message: String) {
        self.diags.push(Diagnostic {
            code,
            severity: Severity::Error,
            span,
            message,
            path: Vec::new(),
            related: None,
            node_kind: "",
        });
    }

    fn value_error(&mut self, node: Node<'_>, d: ValueDiagnostic) {
        self.diags.push(Diagnostic {
            code: d.code,
            severity: Severity::Error,
            span: span(node),
            message: d.message,
            path: d.path,
            related: None,
            node_kind: match node.kind() {
                "tagged_string" => "tagged_string",
                "hexadecimal" => "hexadecimal",
                "octal" => "octal",
                "integer" => "integer",
                "decimal" => "decimal",
                "measurement" => "measurement",
                "range" => "range",
                "map" => "map",
                _ => "",
            },
        });
    }

    fn doc(&mut self, n: Node<'_>) {
        self.docs.push(DocComment {
            span: span(n),
            text: self.text(n).to_string(),
        });
    }

    fn content(&self, n: Node<'_>) -> &'a str {
        n.child_by_field_name("content")
            .map(|c| self.text(c))
            .unwrap_or("")
    }

    fn name(&self, n: Node<'_>) -> Name {
        let (text, style) = match n.kind() {
            "quoted_name" => (unescape(self.content_of_name(n), '`'), NameStyle::Backtick),
            "double_quoted_name" => (unescape(self.content_of_name(n), '"'), NameStyle::DoubleQuote),
            "integer" => (self.text(n).to_string(), NameStyle::Integer),
            _ => (self.text(n).to_string(), NameStyle::Symbol),
        };
        Name {
            text,
            span: span(n),
            style,
        }
    }

    /// The content of a quoted name: the text between the delimiters.
    fn content_of_name(&self, n: Node<'_>) -> &'a str {
        let t = self.text(n);
        &t[1..t.len().saturating_sub(1).max(1)]
    }

    fn record(&mut self, n: Node<'_>) -> Record {
        let mut props: Vec<Property> = Vec::new();
        let mut cursor = n.walk();
        for p in n.named_children(&mut cursor) {
            if p.kind() != "record_property" {
                continue;
            }
            let (Some(k), Some(v)) = (p.child_by_field_name("key"), p.child_by_field_name("value")) else {
                continue;
            };
            let key = self.name(k);
            let (double, colon_span) = {
                let mut c = p.walk();
                let tok = p.children(&mut c).find(|t| t.kind() == ":" || t.kind() == "::");
                (
                    tok.map(|t| t.kind() == "::").unwrap_or(false),
                    tok.map(span).unwrap_or(span(p)),
                )
            };
            let value = self.value(v);
            if let Some(first) = props.iter().find(|q| q.key.text == key.text) {
                let first_span = first.key.span;
                self.diags.push(Diagnostic {
                    code: "duplicate-key",
                    severity: Severity::Error,
                    span: key.span,
                    message: format!("the key `{}` is written more than once", key.text),
                    path: Vec::new(),
                    related: Some(("first written here", first_span)),
                    node_kind: "",
                });
            }
            props.push(Property {
                span: span(p),
                key,
                double_colon: double,
                colon_span,
                value,
            });
        }
        Record {
            span: Some(span(n)),
            properties: props,
        }
    }

    fn subject(&mut self, n: Node<'_>) -> Option<Subject> {
        let id = n.child_by_field_name("identifier");
        let labels = n.child_by_field_name("labels");
        let record = n.child_by_field_name("record");
        if id.is_none() && labels.is_none() && record.is_none() {
            return None;
        }
        Some(Subject {
            span: span(n),
            identifier: id.map(|i| self.name(i)),
            labels: labels.map(|l| self.labels(l)).unwrap_or_default(),
            record: record.map(|r| self.record(r)),
        })
    }

    fn labels(&self, n: Node<'_>) -> Vec<Label> {
        let mut out = Vec::new();
        let mut double = false;
        let mut colon = span(n);
        let mut cursor = n.walk();
        for c in n.children(&mut cursor) {
            match c.kind() {
                ":" => {
                    double = false;
                    colon = span(c);
                }
                "::" => {
                    double = true;
                    colon = span(c);
                }
                "symbol" | "quoted_name" => out.push(Label {
                    name: self.name(c),
                    double_colon: double,
                    colon_span: colon,
                }),
                _ => {}
            }
        }
        out
    }

    fn element(&mut self, n: Node<'_>) -> Option<Element> {
        match n.kind() {
            "node_pattern" => Some(Element::Node {
                span: span(n),
                subject: self.subject(n),
            }),
            "relationship_pattern" => {
                let left = self.element(n.child_by_field_name("left")?)?;
                let arrow = self.arrow(n.child_by_field_name("kind")?);
                let right = self.element(n.child_by_field_name("right")?)?;
                Some(Element::Relationship {
                    span: span(n),
                    left: Box::new(left),
                    arrow,
                    right: Box::new(right),
                })
            }
            "subject_pattern" => {
                let mut elements = Vec::new();
                let mut cursor = n.walk();
                for c in n.named_children(&mut cursor) {
                    if c.kind() == "subject_pattern_elements" {
                        let mut c2 = c.walk();
                        for e in c.named_children(&mut c2) {
                            if let Some(el) = self.element(e) {
                                elements.push(el);
                            }
                        }
                    }
                }
                Some(Element::SubjectPattern {
                    span: span(n),
                    subject: self.subject(n),
                    elements,
                })
            }
            "annotated_pattern" => {
                let a = n.child_by_field_name("annotations")?;
                let annotations = self.annotations(a);
                let element = self.element(n.child_by_field_name("elements")?)?;
                Some(Element::Annotated {
                    span: span(n),
                    annotations,
                    element: Box::new(element),
                })
            }
            "pattern_reference" => {
                let id = n.child_by_field_name("identifier")?;
                Some(Element::Reference {
                    span: span(n),
                    identifier: self.name(id),
                })
            }
            "doc_comment" => {
                self.doc(n);
                None
            }
            _ => None,
        }
    }

    fn arrow(&mut self, n: Node<'_>) -> Arrow {
        let direction = match n.kind() {
            "undirected_arrow" => Direction::Undirected,
            "right_arrow" => Direction::Right,
            "left_arrow" => Direction::Left,
            _ => Direction::Both,
        };
        let t = self.text(n);
        let first = t.trim_start_matches('<').chars().next().unwrap_or('-');
        let family = match first {
            '=' => ArrowFamily::Equals,
            '~' => ArrowFamily::Tilde,
            _ => ArrowFamily::Dash,
        };
        Arrow {
            span: span(n),
            family,
            direction,
            subject: self.subject(n),
        }
    }

    fn annotations(&mut self, n: Node<'_>) -> Annotations {
        let mut identified = None;
        let mut properties = Vec::new();
        let mut cursor = n.walk();
        for c in n.named_children(&mut cursor) {
            match c.kind() {
                "identified_annotation" => {
                    let id = c.child_by_field_name("identifier").map(|i| self.name(i));
                    let labels = c.child_by_field_name("labels").map(|l| self.labels(l)).unwrap_or_default();
                    identified = Some((id, labels));
                }
                "property_annotation" => {
                    if let (Some(k), Some(v)) = (c.child_by_field_name("key"), c.child_by_field_name("value")) {
                        properties.push(AnnotationProperty {
                            span: span(c),
                            key: self.name(k),
                            value: self.value(v),
                        });
                    }
                }
                _ => {}
            }
        }
        Annotations {
            span: span(n),
            identified,
            properties,
        }
    }

    // --- values -------------------------------------------------------------------------

    fn lowered(&self, n: Node<'_>, value: Value, node_kind: &'static str, parts: Vec<Span>) -> Lowered {
        Lowered {
            value,
            span: span(n),
            node_kind,
            parts,
        }
    }

    fn value(&mut self, n: Node<'_>) -> Lowered {
        let kind: &'static str = match n.kind() {
            "integer" => "integer",
            "decimal" => "decimal",
            "hexadecimal" => "hexadecimal",
            "octal" => "octal",
            "measurement" => "measurement",
            "range" => "range",
            "boolean_literal" => "boolean_literal",
            "symbol" => "symbol",
            "string_literal" => "string_literal",
            "tagged_string" => "tagged_string",
            "array" => "array",
            "map" => "map",
            _ => "unknown",
        };
        let text = self.text(n);
        let mut parts = Vec::new();
        let value = match kind {
            "integer" | "decimal" | "hexadecimal" | "octal" | "measurement" => self.number(n, kind, text),
            "boolean_literal" => Value::boolean(text == "true"),
            "symbol" => Value::string(text),
            "string_literal" => Value::string(self.string(n, text)),
            "tagged_string" => self.tagged(n),
            "range" => {
                let lower = n.child_by_field_name("lower").map(|l| self.number(l, l.kind(), self.text(l)));
                let upper = n.child_by_field_name("upper").map(|u| self.number(u, u.kind(), self.text(u)));
                match Value::range(lower, upper) {
                    Ok(v) => v,
                    Err(d) => {
                        self.value_error(n, d);
                        Value::string(text)
                    }
                }
            }
            "array" => {
                let mut items = Vec::new();
                let mut cursor = n.walk();
                for c in n.named_children(&mut cursor) {
                    if matches!(c.kind(), "comment" | "doc_comment") {
                        continue;
                    }
                    let l = self.value(c);
                    parts.push(l.span);
                    items.push(l.value);
                }
                Value::array(items)
            }
            "map" => {
                let mut entries: Vec<(String, Value)> = Vec::new();
                let mut cursor = n.walk();
                for e in n.named_children(&mut cursor) {
                    if e.kind() != "map_entry" {
                        continue;
                    }
                    let (Some(k), Some(v)) = (e.child_by_field_name("key"), e.child_by_field_name("value")) else {
                        continue;
                    };
                    let key = self.name(k);
                    let l = self.value(v);
                    if let Some(_) = entries.iter().find(|(q, _)| *q == key.text) {
                        let first = parts.get(entries.iter().position(|(q, _)| *q == key.text).unwrap()).copied();
                        self.diags.push(Diagnostic {
                            code: "duplicate-key",
                            severity: Severity::Error,
                            span: key.span,
                            message: format!("the key `{}` is written more than once", key.text),
                            path: vec![PathSegment::Key(key.text.clone())],
                            related: first.map(|f| ("first written here", f)),
                            node_kind: "",
                        });
                        continue;
                    }
                    parts.push(l.span);
                    entries.push((key.text, l.value));
                }
                match Value::map(entries) {
                    Ok(v) => v,
                    Err(d) => {
                        self.value_error(n, d);
                        Value::string(text)
                    }
                }
            }
            _ => Value::string(text),
        };
        self.lowered(n, value, kind, parts)
    }

    fn string(&self, n: Node<'_>, text: &str) -> String {
        let raw = self.content(n);
        if text.starts_with("```") {
            raw.to_string()
        } else {
            unescape(raw, text.chars().next().unwrap_or('"'))
        }
    }

    fn tagged(&mut self, n: Node<'_>) -> Value {
        let tag = n.child_by_field_name("tag").map(|t| self.text(t)).unwrap_or("");
        let raw = self.content(n);
        let content = if self.text(n).starts_with("```") {
            raw.to_string()
        } else {
            unescape(raw, '`')
        };
        // gram-level meaning of a known tag: report bad content, keep the value as written.
        if let Err(d) = interpret_tag(tag, &content) {
            self.value_error(n, d);
        }
        match Value::tagged(tag, content.clone()) {
            Ok(v) => v,
            Err(d) => {
                self.value_error(n, d);
                Value::string(content)
            }
        }
    }

    fn number(&mut self, n: Node<'_>, kind: &str, text: &str) -> Value {
        let result: Result<Value, ValueDiagnostic> = match kind {
            "integer" if text == "-0" => Ok(Value::integer(0)),
            "integer" => Value::integer_from_digits(text),
            "decimal" => Value::decimal_from_text(text),
            "hexadecimal" => radix(&text[2..], 16, text),
            "octal" => radix(&text[1..], 8, text),
            "measurement" => {
                let split = text
                    .find(|c: char| c.is_ascii_alphabetic())
                    .unwrap_or(text.len());
                let (mag, unit) = text.split_at(split);
                let magnitude = if mag.contains('.') {
                    Value::decimal_from_text(mag)
                } else if mag == "-0" {
                    Ok(Value::integer(0))
                } else {
                    Value::integer_from_digits(mag)
                };
                magnitude.and_then(|m| Value::measurement(m, unit))
            }
            _ => Ok(Value::string(text)),
        };
        match result {
            Ok(v) => v,
            Err(d) => {
                self.value_error(n, d);
                Value::string(text)
            }
        }
    }
}

fn radix(digits: &str, base: u32, whole: &str) -> Result<Value, ValueDiagnostic> {
    match i64::from_str_radix(digits, base) {
        Ok(i) => Ok(Value::integer(i)),
        Err(_) => Err(ValueDiagnostic::new(
            "out-of-range",
            format!("the integer `{whole}` does not fit in 64 bits"),
        )),
    }
}
