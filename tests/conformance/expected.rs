//! `expected.gram` (ADR-015 section 2.3): an ordinary gram file under rules that make it read the
//! same in every gram library. The runner enforces them and fails the case on a breach.
//!
//! - every node is defined once, at top level, with its labels and properties;
//! - a relationship is written singly, its end nodes named by identifier alone and defined earlier;
//! - no paths, forward references, annotations, header record or map values;
//! - an identifier is the element's `@id`, unless the case names an identity property for that kind,
//!   in which case it is only a handle and the property is written in the record.
use super::graph::{node_line, relationship_line, Graph};
use super::value::expected_value;
use gram_codec::parse_gram_with_header;
use gram_value::Value;
use pattern_core::{Pattern, Subject};
use std::collections::{BTreeMap, HashMap};

/// Which kinds of element take their identity from a named property (the case's settings).
#[derive(Clone, Debug, Default)]
pub struct Identity {
    pub node_property: Option<String>,
    pub relationship_property: Option<String>,
}

fn props_of(
    s: &Subject,
    id_property: &Option<String>,
    as_id: bool,
) -> Result<BTreeMap<String, Value>, String> {
    let mut out = BTreeMap::new();
    for (k, v) in &s.properties {
        out.insert(
            k.clone(),
            expected_value(v).map_err(|e| format!("property `{k}`: {e}"))?,
        );
    }
    if as_id && id_property.is_none() && !s.identity.0.is_empty() {
        out.insert("@id".to_string(), Value::string(s.identity.0.clone()));
    }
    Ok(out)
}

fn labels_of(s: &Subject) -> Vec<String> {
    s.labels.iter().cloned().collect()
}

fn is_reference(p: &Pattern<Subject>) -> bool {
    p.elements.is_empty()
        && !p.value.identity.0.is_empty()
        && p.value.labels.is_empty()
        && p.value.properties.is_empty()
}

pub fn read_expected(text: &str, identity: &Identity) -> Result<Graph, String> {
    let (header, patterns) =
        parse_gram_with_header(text).map_err(|e| format!("expected.gram does not parse: {e}"))?;
    if header.is_some() {
        return Err(
            "expected.gram has a header record; it holds only nodes and relationships".into(),
        );
    }
    let mut nodes: Vec<String> = Vec::new();
    let mut defined: HashMap<String, String> = HashMap::new();
    let mut relationships: Vec<String> = Vec::new();
    // identifiers that appear as a relationship end before their definition
    let mut later_defined: Vec<String> = Vec::new();
    for p in &patterns {
        match p.elements.len() {
            0 => {
                let s = &p.value;
                let line = node_line(&labels_of(s), &props_of(s, &identity.node_property, true)?)?;
                if !s.identity.0.is_empty()
                    && defined.insert(s.identity.0.clone(), line.clone()).is_some()
                {
                    return Err(format!("`{}` is defined more than once", s.identity.0));
                }
                if later_defined.contains(&s.identity.0) {
                    return Err(format!(
                        "`{}` is a forward reference: define a node before a relationship uses it",
                        s.identity.0
                    ));
                }
                nodes.push(line);
            }
            1 => return Err(
                "an annotation is not allowed in expected.gram (it reads as a one-element pattern)"
                    .into(),
            ),
            2 => {
                let (a, b) = (&p.elements[0], &p.elements[1]);
                for end in [a, b] {
                    if !end.elements.is_empty() {
                        return Err("a path is not allowed in expected.gram: write each relationship on its own".into());
                    }
                    if !is_reference(end) {
                        return Err("a relationship names its end nodes by identifier alone; define the node at top level".into());
                    }
                }
                let (start, stop) = match (
                    defined.get(&a.value.identity.0),
                    defined.get(&b.value.identity.0),
                ) {
                    (Some(x), Some(y)) => (x.clone(), y.clone()),
                    _ => {
                        for end in [a, b] {
                            if !defined.contains_key(&end.value.identity.0) {
                                later_defined.push(end.value.identity.0.clone());
                            }
                        }
                        let missing = if !defined.contains_key(&a.value.identity.0) {
                            &a.value.identity.0
                        } else {
                            &b.value.identity.0
                        };
                        return Err(format!(
                            "the end `{missing}` is not defined before the relationship (a forward reference, or not defined at top level)"
                        ));
                    }
                };
                let s = &p.value;
                let ty = match labels_of(s).as_slice() {
                    [t] => t.clone(),
                    other => {
                        return Err(format!(
                            "a relationship has exactly one type, found {other:?}"
                        ))
                    }
                };
                relationships.push(relationship_line(
                    &ty,
                    &props_of(s, &identity.relationship_property, true)?,
                    &start,
                    &stop,
                )?);
            }
            n => {
                return Err(format!(
                    "a pattern in expected.gram has at most two elements, found {n}"
                ))
            }
        }
    }
    Ok(Graph::new(nodes, relationships))
}
