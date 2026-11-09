//! The load plan: the file's elements in order of first appearance, with the
//! file-scope rules applied (ADR-001 sections 2.1 to 2.8, ADR-003, ADR-007
//! section 2.4, ADR-008).

use crate::diagnostics::{code, Diagnostic, Span};
use crate::settings::LoadSettings;
use crate::syntax::Syntax;
use crate::values::{convert_record, name_text, string_value, PropValue};
use std::collections::HashMap;
use tree_sitter::Node;

/// The only file kind the loader supports (ADR-003 section 2.1).
const SUPPORTED_KIND: &str = "property-graph";

#[derive(Clone, Debug)]
pub struct PlannedNode {
    /// The identifier text, or `None` for an anonymous node.
    pub key: Option<String>,
    pub labels: Vec<String>,
    pub props: Vec<(String, PropValue)>,
    /// The definition's span, or the first reference's if it is never defined.
    pub span: Span,
    pub defined: bool,
    /// The identifier's span in the defining appearance.
    def_ident: Option<Span>,
}

#[derive(Clone, Debug)]
pub struct PlannedRel {
    pub key: Option<String>,
    /// `None` until a definition supplies the type.
    pub rel_type: Option<String>,
    /// Indexes into `LoadPlan::nodes`.
    pub start: usize,
    pub end: usize,
    pub props: Vec<(String, PropValue)>,
    pub span: Span,
    pub defined: bool,
    def_ident: Option<Span>,
    first_ident: Option<Span>,
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Node,
    Relationship,
}

struct Builder<'a> {
    syn: &'a Syntax,
    settings: &'a LoadSettings,
    plan: LoadPlan,
    node_ids: HashMap<String, usize>,
    rel_ids: HashMap<String, usize>,
    kinds: HashMap<String, (Kind, Span)>,
}

pub fn build(syn: &Syntax, settings: &LoadSettings) -> LoadPlan {
    let mut b = Builder {
        syn,
        settings,
        plan: LoadPlan::default(),
        node_ids: HashMap::new(),
        rel_ids: HashMap::new(),
        kinds: HashMap::new(),
    };
    b.check_file_name();
    scan_constructs(syn, syn.root(), &mut b.plan.diagnostics);
    let root = syn.root();
    let mut cursor = root.walk();
    for child in root.named_children(&mut cursor) {
        match child.kind() {
            "comment" | "doc_comment" => {}
            "record" => b.record(child),
            "node_pattern" => {
                b.node_appearance(child);
            }
            "relationship_pattern" => b.chain(child),
            // Reported by `scan_constructs`.
            "annotated_pattern" | "subject_pattern" | "pattern_reference" => {}
            other => b.plan.diagnostics.push(Diagnostic::error(
                code::CONSTRUCT_UNSUPPORTED,
                syn.span(child),
                format!("the construct `{other}` is not supported"),
            )),
        }
    }
    b.finish()
}

/// Report annotations, subject patterns and double colons anywhere in the tree
/// (ADR-007 section 2.4).
fn scan_constructs(syn: &Syntax, node: Node<'_>, diags: &mut Vec<Diagnostic>) {
    match node.kind() {
        "annotated_pattern" => {
            diags.push(Diagnostic::error(
                code::CONSTRUCT_ANNOTATION,
                syn.span(node),
                "annotations are not supported",
            ));
            return;
        }
        "subject_pattern" | "pattern_reference" => {
            diags.push(Diagnostic::error(
                code::CONSTRUCT_SUBJECT_PATTERN,
                syn.span(node),
                "subject patterns are not supported",
            ));
            return;
        }
        "::" => {
            diags.push(Diagnostic::error(
                code::CONSTRUCT_DOUBLE_COLON,
                syn.span(node),
                "a double colon is not supported; write a single colon",
            ));
            return;
        }
        _ => {}
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        scan_constructs(syn, child, diags);
    }
}

/// The family of an arrow: `-`, `=` or `~`, read from the arrow's text.
fn arrow_family(text: &str) -> char {
    let mut chars = text.chars();
    match chars.next() {
        Some('<') => chars.next().unwrap_or('-'),
        Some(c) => c,
        None => '-',
    }
}

impl<'a> Builder<'a> {
    fn identifier(&self, owner: Node<'_>) -> Option<(String, Span)> {
        owner
            .child_by_field_name("identifier")
            .map(|n| (name_text(self.syn, n), self.syn.span(n)))
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

    /// The kind of file named by its file name, `graph-name.kind.gram` (ADR-003 section 2.1).
    fn check_file_name(&mut self) {
        let Some(name) = &self.settings.file_name else {
            return;
        };
        let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
        let stem = base.strip_suffix(".gram").unwrap_or(base);
        if let Some((_, kind)) = stem.rsplit_once('.') {
            if kind != SUPPORTED_KIND {
                self.plan.diagnostics.push(Diagnostic::error_without_span(
                    code::FILE_KIND,
                    kind_message(kind),
                ));
            }
        }
    }

    /// The top-level record: check `kind`, check the values, report what is not stored (ADR-003).
    fn record(&mut self, record: Node<'_>) {
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
            if name_text(self.syn, key) != "kind" {
                continue;
            }
            let kind = match value.kind() {
                "string_literal" => Some(string_value(self.syn, value)),
                "symbol" => Some(self.syn.text(value).to_string()),
                _ => None,
            };
            if kind.as_deref() != Some(SUPPORTED_KIND) {
                let shown = kind.unwrap_or_else(|| self.syn.text(value).to_string());
                self.plan.diagnostics.push(Diagnostic::error(
                    code::FILE_KIND,
                    self.syn.span(value),
                    kind_message(&shown),
                ));
            }
        }
        let stored = convert_record(
            self.syn,
            record,
            self.settings.tags,
            &mut self.plan.diagnostics,
        );
        let keys: Vec<&str> = stored
            .iter()
            .map(|(k, _)| k.as_str())
            .filter(|k| *k != "kind")
            .collect();
        if !keys.is_empty() {
            self.plan.diagnostics.push(Diagnostic::warning(
                code::FILE_RECORD_NOT_STORED,
                self.syn.span(record),
                format!(
                    "the record's properties are not stored: {}",
                    keys.join(", ")
                ),
            ));
        }
    }

    /// Record the kind an identifier names; a conflicting use is an error.
    /// Returns false on a conflict.
    fn claim(&mut self, key: &str, kind: Kind, ident_span: Span) -> bool {
        match self.kinds.get(key) {
            Some((k, first)) if *k != kind => {
                let was = if *k == Kind::Node {
                    "a node"
                } else {
                    "a relationship"
                };
                self.plan.diagnostics.push(
                    Diagnostic::error(
                        code::ID_KIND_CONFLICT,
                        ident_span,
                        format!(
                            "`{key}` was first used as {was} and cannot be used as the other kind"
                        ),
                    )
                    .with_related("first used here", *first),
                );
                false
            }
            Some(_) => true,
            None => {
                self.kinds.insert(key.to_string(), (kind, ident_span));
                true
            }
        }
    }

    fn new_node(&mut self, key: Option<String>, span: Span) -> usize {
        self.plan.nodes.push(PlannedNode {
            key,
            labels: Vec::new(),
            props: Vec::new(),
            span,
            defined: false,
            def_ident: None,
        });
        self.plan.nodes.len() - 1
    }

    /// One appearance of a node; returns its index in the plan.
    fn node_appearance(&mut self, node: Node<'_>) -> usize {
        let ident = self.identifier(node);
        let has_labels = node.child_by_field_name("labels").is_some();
        let record = node.child_by_field_name("record");
        let defined = has_labels || record.is_some();
        let span = self.syn.span(node);

        let (key, ident_span) = match ident {
            Some((k, s)) => (Some(k), Some(s)),
            None => (None, None),
        };
        if let (Some(k), Some(s)) = (&key, ident_span) {
            if !self.claim(k, Kind::Node, s) {
                // The error aborts the load; keep an index for the chain.
                return self.new_node(None, span);
            }
        }
        let idx = match key.as_ref().and_then(|k| self.node_ids.get(k)).copied() {
            Some(i) => i,
            None => {
                let i = self.new_node(key.clone(), span);
                if let Some(k) = key.clone() {
                    self.node_ids.insert(k, i);
                }
                i
            }
        };
        if !defined {
            return idx;
        }
        if self.plan.nodes[idx].defined {
            let first = self.plan.nodes[idx].def_ident;
            let mut d = Diagnostic::error(
                code::ID_DUPLICATE_DEFINITION,
                ident_span.unwrap_or(span),
                "this node is defined twice; define it once and refer to it elsewhere as `(id)`",
            );
            if let Some(f) = first {
                d = d.with_related("first defined here", f);
            }
            self.plan.diagnostics.push(d);
            return idx;
        }
        let labels = self.label_names(node);
        let props = match record {
            Some(r) => convert_record(self.syn, r, self.settings.tags, &mut self.plan.diagnostics),
            None => Vec::new(),
        };
        let n = &mut self.plan.nodes[idx];
        n.labels = labels;
        n.props = props;
        n.span = span;
        n.defined = true;
        n.def_ident = ident_span;
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
        let span = self.syn.span(arrow);
        let kind = arrow.kind();
        // Arrow forms (ADR-008 sections 2.1 and 2.2).
        if arrow_family(self.syn.text(arrow)) != '-' {
            self.plan.diagnostics.push(Diagnostic::error(
                code::REL_ARROW_FAMILY,
                span,
                "fat and wavy arrows are not data relationships; use a thin arrow (`-->`)",
            ));
            return;
        }
        match kind {
            "undirected_arrow" => {
                self.plan.diagnostics.push(Diagnostic::error(
                    code::REL_UNDIRECTED,
                    span,
                    "an undirected arrow has no direction in Cypher; write a right or left arrow",
                ));
                return;
            }
            "bidirectional_arrow" => {
                self.plan.diagnostics.push(Diagnostic::error(
                    code::REL_BIDIRECTIONAL,
                    span,
                    "a bidirectional arrow has no direction in Cypher; write a right or left arrow",
                ));
                return;
            }
            _ => {}
        }
        let (start, end) = if kind == "left_arrow" {
            (right, left)
        } else {
            (left, right)
        };
        let ident = self.identifier(arrow);
        let labels = self.label_names(arrow);
        let record = arrow.child_by_field_name("record");
        let has_definition_content = !labels.is_empty() || record.is_some();

        // ADR-008 section 2.3.
        if labels.len() > 1 {
            self.plan.diagnostics.push(Diagnostic::error(
                code::REL_MANY_LABELS,
                span,
                format!(
                    "a relationship has exactly one type; this one has {}: {}",
                    labels.len(),
                    labels.join(", ")
                ),
            ));
            return;
        }
        if labels.is_empty() && (record.is_some() || ident.is_none()) {
            self.plan.diagnostics.push(Diagnostic::error(
                code::REL_NO_TYPE,
                span,
                "a relationship needs a type; write one, for example `-[:KNOWS]->`",
            ));
            return;
        }

        let (key, ident_span) = match ident {
            Some((k, s)) => (Some(k), Some(s)),
            None => (None, None),
        };
        if let (Some(k), Some(s)) = (&key, ident_span) {
            if !self.claim(k, Kind::Relationship, s) {
                return;
            }
        }
        let existing = key.as_ref().and_then(|k| self.rel_ids.get(k)).copied();
        let idx = match existing {
            Some(i) => {
                let r = &self.plan.rels[i];
                if (r.start, r.end) != (start, end) {
                    let mut d = Diagnostic::error(
                        code::ID_ENDPOINTS_DIFFER,
                        ident_span.unwrap_or(span),
                        "this relationship connects different nodes than its first appearance",
                    );
                    if let Some(f) = r.first_ident {
                        d = d.with_related("first written here", f);
                    }
                    self.plan.diagnostics.push(d);
                    return;
                }
                i
            }
            None => {
                self.plan.rels.push(PlannedRel {
                    key: key.clone(),
                    rel_type: None,
                    start,
                    end,
                    props: Vec::new(),
                    span,
                    defined: false,
                    def_ident: None,
                    first_ident: ident_span,
                });
                let i = self.plan.rels.len() - 1;
                if let Some(k) = key {
                    self.rel_ids.insert(k, i);
                }
                i
            }
        };
        if !has_definition_content {
            return; // a reference
        }
        if self.plan.rels[idx].defined {
            let first = self.plan.rels[idx].def_ident;
            let mut d = Diagnostic::error(
                code::ID_DUPLICATE_DEFINITION,
                ident_span.unwrap_or(span),
                "this relationship is defined twice; define it once and refer to it elsewhere as `[id]`",
            );
            if let Some(f) = first {
                d = d.with_related("first defined here", f);
            }
            self.plan.diagnostics.push(d);
            return;
        }
        let props = match record {
            Some(r) => convert_record(self.syn, r, self.settings.tags, &mut self.plan.diagnostics),
            None => Vec::new(),
        };
        let rel = &mut self.plan.rels[idx];
        rel.rel_type = labels.into_iter().next();
        rel.props = props;
        rel.span = span;
        rel.defined = true;
        rel.def_ident = ident_span;
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
        for r in &self.plan.rels {
            if let (Some(key), None) = (&r.key, &r.rel_type) {
                self.plan.diagnostics.push(Diagnostic::error(
                    code::ID_REL_NEVER_DEFINED,
                    r.first_ident.unwrap_or(r.span),
                    format!("the relationship `{key}` is referenced but never defined, so it has no type"),
                ));
            }
        }
        self.plan
            .diagnostics
            .sort_by_key(|d| d.span.map(|s| s.start).unwrap_or(usize::MAX));
        self.plan
    }
}

fn kind_message(kind: &str) -> String {
    format!("relate-gram loads only kind '{SUPPORTED_KIND}'; this file is kind '{kind}'")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_arrow_family_is_read_from_the_arrow_text() {
        assert_eq!(arrow_family("-->"), '-');
        assert_eq!(arrow_family("<--"), '-');
        assert_eq!(arrow_family("-[:T]->"), '-');
        assert_eq!(arrow_family("<-[:T]->"), '-');
        assert_eq!(arrow_family("==>"), '=');
        assert_eq!(arrow_family("<=[:T]="), '=');
        assert_eq!(arrow_family("<~~>"), '~');
        assert_eq!(arrow_family("~[:T]~>"), '~');
    }

    // `construct.unsupported` for an unknown node kind (ADR-007 section 2.4) is a
    // guard against a grammar upgrade. A tree-sitter node cannot be built by hand,
    // and no input of tree-sitter-gram 0.6.1 reaches it, so it has no test.
}
