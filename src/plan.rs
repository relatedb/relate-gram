//! The load plan: the file's elements in order of first appearance, with the
//! file-scope identity rules applied (ADR-001 sections 2.1 to 2.6, ADR-008).

use crate::diagnostics::{code, Diagnostic, Span};
use crate::settings::LoadSettings;
use crate::syntax::Syntax;
use crate::values::{convert_record, name_text, PropValue};
use std::collections::HashMap;
use tree_sitter::Node;

#[derive(Clone, Debug)]
pub struct PlannedNode {
    /// The identifier text, or `None` for an anonymous node.
    pub key: Option<String>,
    pub labels: Vec<String>,
    pub props: Vec<(String, PropValue)>,
    /// The definition's span, or the first reference's if it is never defined.
    pub span: Span,
    pub defined: bool,
}

#[derive(Clone, Debug)]
pub struct PlannedRel {
    pub key: Option<String>,
    pub rel_type: String,
    /// Indexes into `LoadPlan::nodes`.
    pub start: usize,
    pub end: usize,
    pub props: Vec<(String, PropValue)>,
    pub span: Span,
    pub defined: bool,
}

#[derive(Debug, Default)]
pub struct LoadPlan {
    pub nodes: Vec<PlannedNode>,
    pub rels: Vec<PlannedRel>,
    pub diagnostics: Vec<Diagnostic>,
}

impl LoadPlan {
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(Diagnostic::is_error)
    }
}

struct Builder<'a> {
    syn: &'a Syntax,
    settings: &'a LoadSettings,
    plan: LoadPlan,
    node_ids: HashMap<String, usize>,
    rel_ids: HashMap<String, usize>,
}

pub fn build(syn: &Syntax, settings: &LoadSettings) -> LoadPlan {
    let mut b = Builder {
        syn,
        settings,
        plan: LoadPlan::default(),
        node_ids: HashMap::new(),
        rel_ids: HashMap::new(),
    };
    let root = syn.root();
    let mut cursor = root.walk();
    for child in root.named_children(&mut cursor) {
        match child.kind() {
            "comment" | "doc_comment" => {}
            // The top-level record is handled with the file rules (ADR-003).
            "record" => {}
            "node_pattern" => {
                b.node_appearance(child);
            }
            "relationship_pattern" => b.chain(child),
            other => b.plan.diagnostics.push(Diagnostic::error(
                code::CONSTRUCT_UNSUPPORTED,
                syn.span(child),
                format!("the construct `{other}` is not supported"),
            )),
        }
    }
    b.finish()
}

impl<'a> Builder<'a> {
    fn identifier(&self, owner: Node<'_>) -> Option<String> {
        owner
            .child_by_field_name("identifier")
            .map(|n| name_text(self.syn, n))
    }

    fn label_names(&self, owner: Node<'_>) -> Vec<String> {
        let Some(labels) = owner.child_by_field_name("labels") else {
            return Vec::new();
        };
        let mut cursor = labels.walk();
        labels
            .named_children(&mut cursor)
            .filter(|n| matches!(n.kind(), "symbol" | "quoted_name"))
            .map(|n| name_text(self.syn, n))
            .collect()
    }

    /// One appearance of a node; returns its index in the plan.
    fn node_appearance(&mut self, node: Node<'_>) -> usize {
        let key = self.identifier(node);
        let has_labels = node.child_by_field_name("labels").is_some();
        let record = node.child_by_field_name("record");
        let defined = has_labels || record.is_some();
        let span = self.syn.span(node);

        let idx = match key.as_ref().and_then(|k| self.node_ids.get(k)).copied() {
            Some(i) => i,
            None => {
                self.plan.nodes.push(PlannedNode {
                    key: key.clone(),
                    labels: Vec::new(),
                    props: Vec::new(),
                    span,
                    defined: false,
                });
                let i = self.plan.nodes.len() - 1;
                if let Some(k) = key {
                    self.node_ids.insert(k, i);
                }
                i
            }
        };
        // A second definition is an error under ADR-001 section 2.3; until the
        // rejection rules are added the first definition wins.
        if defined && !self.plan.nodes[idx].defined {
            let labels = self.label_names(node);
            let props = match record {
                Some(r) => {
                    convert_record(self.syn, r, self.settings.tags, &mut self.plan.diagnostics)
                }
                None => Vec::new(),
            };
            let n = &mut self.plan.nodes[idx];
            n.labels = labels;
            n.props = props;
            n.span = span;
            n.defined = true;
        }
        idx
    }

    /// A relationship pattern and the chain to its right, in order of appearance.
    fn chain(&mut self, first: Node<'_>) {
        let mut rel_node = first;
        let mut current = match rel_node.child_by_field_name("left") {
            Some(l) => self.node_appearance(l),
            None => return,
        };
        loop {
            let (Some(arrow), Some(right)) = (
                rel_node.child_by_field_name("kind"),
                rel_node.child_by_field_name("right"),
            ) else {
                return;
            };
            let (next_node, continues) = if right.kind() == "relationship_pattern" {
                match right.child_by_field_name("left") {
                    Some(l) => (l, true),
                    None => return,
                }
            } else {
                (right, false)
            };
            let next = self.node_appearance(next_node);
            self.relationship_appearance(arrow, current, next);
            if !continues {
                return;
            }
            current = next;
            rel_node = right;
        }
    }

    fn relationship_appearance(&mut self, arrow: Node<'_>, left: usize, right: usize) {
        let (start, end) = if arrow.kind() == "left_arrow" {
            (right, left)
        } else {
            (left, right)
        };
        let key = self.identifier(arrow);
        let labels = self.label_names(arrow);
        let record = arrow.child_by_field_name("record");
        let defined = !labels.is_empty() || record.is_some();
        let span = self.syn.span(arrow);

        if let Some(i) = key.as_ref().and_then(|k| self.rel_ids.get(k)).copied() {
            // A repeat of a relationship identifier binds to the first appearance.
            let _ = i;
            return;
        }
        let Some(rel_type) = labels.first().cloned() else {
            self.plan.diagnostics.push(Diagnostic::error(
                code::REL_NO_TYPE,
                span,
                "a relationship needs a type; write one, for example `-[:KNOWS]->`",
            ));
            return;
        };
        let props = match record {
            Some(r) => convert_record(self.syn, r, self.settings.tags, &mut self.plan.diagnostics),
            None => Vec::new(),
        };
        self.plan.rels.push(PlannedRel {
            key: key.clone(),
            rel_type,
            start,
            end,
            props,
            span,
            defined,
        });
        if let Some(k) = key {
            self.rel_ids.insert(k, self.plan.rels.len() - 1);
        }
    }

    fn finish(mut self) -> LoadPlan {
        for n in &self.plan.nodes {
            if let (Some(key), false) = (&n.key, n.defined) {
                self.plan.diagnostics.push(Diagnostic::warning(
                    code::ID_REFERENCE_ONLY,
                    n.span,
                    format!("`{key}` is referenced but never defined; an empty node was created"),
                ));
            }
        }
        self.plan
            .diagnostics
            .sort_by_key(|d| d.span.map(|s| s.start));
        self.plan
    }
}
