//! The `Pattern<Subject>` view of a document, with a source map, and the diagnostics that need
//! it (an element defined twice).
use crate::syntax::*;
use gram_value::Value;
use std::collections::HashMap;

/// A pattern's subject: the data that gram's model keeps. Arrow family, direction and the
/// choice of `[a]` over `(a)` are not here.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PatternSubject {
    pub identity: Option<String>,
    /// Where the identifier was written, for diagnostics that point at it.
    pub identity_span: Option<Span>,
    pub labels: Vec<String>,
    pub properties: Vec<(String, Value)>,
}

/// `Pattern<Subject>` plus the span of the syntax it came from.
#[derive(Debug, Clone, PartialEq)]
pub struct Pattern {
    pub subject: PatternSubject,
    pub elements: Vec<Pattern>,
    pub origin: Span,
}

fn subject_of(s: &Option<Subject>) -> PatternSubject {
    let Some(s) = s else {
        return PatternSubject::default();
    };
    let mut props: Vec<(String, Value)> = Vec::new();
    if let Some(r) = &s.record {
        for p in &r.properties {
            // a repeated key: the last one wins (the repeat is already a diagnostic)
            props.retain(|(k, _)| *k != p.key.text);
            props.push((p.key.text.clone(), p.value.value.clone()));
        }
    }
    PatternSubject {
        identity: s.identifier.as_ref().map(|i| i.text.clone()),
        identity_span: s.identifier.as_ref().map(|i| i.span),
        labels: s.labels.iter().map(|l| l.name.text.clone()).collect(),
        properties: props,
    }
}

fn pattern_of(e: &Element) -> Pattern {
    match e {
        Element::Node { span, subject } => Pattern {
            subject: subject_of(subject),
            elements: vec![],
            origin: *span,
        },
        Element::Reference { span, identifier } => Pattern {
            subject: PatternSubject {
                identity: Some(identifier.text.clone()),
                identity_span: Some(identifier.span),
                ..Default::default()
            },
            elements: vec![],
            origin: *span,
        },
        Element::SubjectPattern {
            span,
            subject,
            elements,
        } => Pattern {
            subject: subject_of(subject),
            elements: elements.iter().map(pattern_of).collect(),
            origin: *span,
        },
        Element::Annotated {
            span,
            annotations,
            element,
        } => {
            let mut s = PatternSubject::default();
            if let Some((id, labels)) = &annotations.identified {
                s.identity = id.as_ref().map(|i| i.text.clone());
                s.identity_span = id.as_ref().map(|i| i.span);
                s.labels = labels.iter().map(|l| l.name.text.clone()).collect();
            }
            for p in &annotations.properties {
                s.properties.push((p.key.text.clone(), p.value.value.clone()));
            }
            Pattern {
                subject: s,
                elements: vec![pattern_of(element)],
                origin: *span,
            }
        }
        Element::Relationship { .. } => {
            // A path `(a)-[r1]->(b)-[r2]->(c)` is the anonymous pattern of its relationships
            // `[ | [r1 | a, b], [r2 | b, c] ]`; a single relationship is just that pattern.
            let mut rels = Vec::new();
            let mut cur = e;
            let mut left_node: Option<Pattern> = None;
            loop {
                let Element::Relationship {
                    span,
                    left,
                    arrow,
                    right,
                } = cur
                else {
                    break;
                };
                let l = left_node.take().unwrap_or_else(|| pattern_of(left));
                let (next_node, next_cur) = match right.as_ref() {
                    Element::Relationship { left: rl, .. } => (pattern_of(rl), Some(right.as_ref())),
                    other => (pattern_of(other), None),
                };
                let elements = match arrow.direction {
                    Direction::Left => vec![next_node.clone(), l],
                    _ => vec![l, next_node.clone()],
                };
                rels.push(Pattern {
                    subject: subject_of(&arrow.subject),
                    elements,
                    origin: *span,
                });
                match next_cur {
                    Some(c) => {
                        left_node = Some(next_node);
                        cur = c;
                    }
                    None => break,
                }
            }
            if rels.len() == 1 {
                rels.pop().unwrap()
            } else {
                Pattern {
                    subject: PatternSubject::default(),
                    elements: rels,
                    origin: e.span(),
                }
            }
        }
    }
}

/// The patterns of a document, and the diagnostics found while projecting: an identified
/// element defined (given labels, a record or elements) more than once.
pub fn project(doc: &Document) -> (Vec<Pattern>, Vec<Diagnostic>) {
    let patterns: Vec<Pattern> = doc.elements.iter().map(pattern_of).collect();
    let mut defined: HashMap<String, Span> = HashMap::new();
    let mut diags = Vec::new();
    fn walk(p: &Pattern, defined: &mut HashMap<String, Span>, diags: &mut Vec<Diagnostic>) {
        let s = &p.subject;
        let has_content = !s.labels.is_empty() || !s.properties.is_empty() || !p.elements.is_empty();
        if let (Some(id), true) = (&s.identity, has_content) {
            let here = s.identity_span.unwrap_or(p.origin);
            match defined.get(id) {
                Some(first) => diags.push(Diagnostic {
                    code: "duplicate-definition",
                    severity: Severity::Error,
                    span: here,
                    message: format!("`{id}` is defined more than once"),
                    path: Vec::new(),
                    related: Some(("first defined here", *first)),
                    node_kind: "",
                }),
                None => {
                    defined.insert(id.clone(), here);
                }
            }
        }
        for e in &p.elements {
            walk(e, defined, diags);
        }
    }
    for p in &patterns {
        walk(p, &mut defined, &mut diags);
    }
    (patterns, diags)
}
