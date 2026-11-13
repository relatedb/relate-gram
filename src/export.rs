//! Exporting a Relate database as gram text (ADR-002, ADR-006 sections 2.5
//! and 2.6, ADR-008 section 2.4).
//!
//! The database is read in one transaction, nodes and relationships in `id()`
//! order. Every problem is collected before any text is produced.

use crate::diagnostics::{code, Diagnostic};
use crate::names;
use crate::settings::{ExportSettings, IdentityMode};
use relate::{Connection, Value};
use std::collections::{BTreeMap, HashMap, HashSet};

struct ExNode {
    id: i64,
    labels: Vec<String>,
    props: BTreeMap<String, Value>,
}

struct ExRel {
    id: i64,
    ty: String,
    props: BTreeMap<String, Value>,
    start: i64,
    end: i64,
}

fn db_error(e: impl std::fmt::Display) -> Diagnostic {
    Diagnostic::error_without_span(code::LOAD_DATABASE_ERROR, e.to_string())
}

fn as_int(v: &Value) -> Option<i64> {
    match v {
        Value::Int(i) => Some(*i),
        _ => None,
    }
}

fn as_map(v: &Value) -> Option<BTreeMap<String, Value>> {
    match v {
        Value::Map(m) => Some((**m).clone()),
        _ => None,
    }
}

fn read_graph(conn: &mut Connection) -> Result<(Vec<ExNode>, Vec<ExRel>), Diagnostic> {
    let node_rows = conn
        .query("MATCH (n) RETURN id(n), labels(n), properties(n) ORDER BY id(n)")
        .map_err(db_error)?
        .rows;
    let mut nodes = Vec::with_capacity(node_rows.len());
    for row in node_rows {
        let (Some(id), Value::List(labels), Some(props)) =
            (as_int(&row[0]), &row[1], as_map(&row[2]))
        else {
            return Err(db_error("unexpected node row"));
        };
        let mut labels: Vec<String> = labels
            .iter()
            .filter_map(|l| match l {
                Value::String(s) => Some(s.to_string()),
                _ => None,
            })
            .collect();
        labels.sort();
        nodes.push(ExNode { id, labels, props });
    }
    let rel_rows = conn
        .query(
            "MATCH (a)-[r]->(b) RETURN id(r), type(r), properties(r), id(a), id(b) ORDER BY id(r)",
        )
        .map_err(db_error)?
        .rows;
    let mut rels = Vec::with_capacity(rel_rows.len());
    for row in rel_rows {
        let (Some(id), Value::String(ty), Some(props), Some(start), Some(end)) = (
            as_int(&row[0]),
            &row[1],
            as_map(&row[2]),
            as_int(&row[3]),
            as_int(&row[4]),
        ) else {
            return Err(db_error("unexpected relationship row"));
        };
        rels.push(ExRel {
            id,
            ty: ty.to_string(),
            props,
            start,
            end,
        });
    }
    Ok((nodes, rels))
}

/// A float as the shortest decimal that reads back as the same value, with a
/// dot and no exponent (ADR-006 section 2.5).
fn float_text(f: f64) -> String {
    let mut s = format!("{f}");
    if !s.contains('.') {
        s.push_str(".0");
    }
    s
}

fn kind_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Int(_) => "integer",
        Value::Float(_) => "float",
        Value::String(_) => "string",
        Value::List(_) => "list",
        Value::Map(_) => "map",
        Value::Node(_) => "node",
        Value::Relationship(_) => "relationship",
        Value::Path(_) => "path",
        Value::Temporal(_) => "temporal value",
        Value::Point(_) => "point",
        Value::Uuid(_) => "uuid",
        Value::Vector { .. } => "vector",
        Value::Bytes(_) => "byte array",
    }
}

/// A scalar as gram text, or the reason it has none.
fn scalar_text(v: &Value) -> Result<String, String> {
    match v {
        Value::Int(i) => Ok(i.to_string()),
        Value::Bool(b) => Ok(b.to_string()),
        Value::Float(f) if f.is_finite() => Ok(float_text(*f)),
        Value::Float(f) => Err(format!("the float {f} has no gram spelling")),
        Value::String(s) if s.contains('\0') => {
            Err("a string containing a NUL character cannot be written".to_string())
        }
        Value::String(s) => Ok(names::string_literal(s)),
        other => Err(format!("a {} has no gram form", kind_name(other))),
    }
}

/// A property value as gram text, or the reason it has none (ADR-006 sections 2.5 and 2.6).
fn value_text(v: &Value) -> Result<String, String> {
    match v {
        Value::List(items) => {
            if items.is_empty() {
                return Ok("[]".to_string());
            }
            let mut parts = Vec::with_capacity(items.len());
            for item in items.iter() {
                if matches!(item, Value::List(_)) {
                    return Err("a nested list has no gram form".to_string());
                }
                parts.push(scalar_text(item)?);
            }
            // Gram arrays hold one kind; Relate stores integers and floats
            // together only by converting to floats, so mixed kinds do not occur,
            // but check rather than rely on it.
            let kinds: HashSet<&str> = items.iter().map(kind_name).collect();
            if kinds.len() > 1 {
                return Err("a list of mixed kinds has no gram form".to_string());
            }
            Ok(format!("[{}]", parts.join(", ")))
        }
        other => scalar_text(other),
    }
}

#[derive(Clone, Copy)]
enum Kind {
    Node,
    Relationship,
}

impl Kind {
    fn label(self, id: i64) -> String {
        match self {
            Kind::Node => format!("node {id}"),
            Kind::Relationship => format!("relationship {id}"),
        }
    }
}

/// Checks one element's properties and renders its record.
fn record_text(
    kind: Kind,
    id: i64,
    props: &BTreeMap<String, Value>,
    errors: &mut Vec<Diagnostic>,
) -> String {
    let mut parts = Vec::new();
    for (key, value) in props {
        if key == "@id" {
            continue; // written as the identifier, never as a key
        }
        if key.is_empty() {
            errors.push(Diagnostic::error_without_span(
                code::EXPORT_VALUE,
                format!("{} has a property with an empty key", kind.label(id)),
            ));
            continue;
        }
        if key.starts_with('@') {
            errors.push(Diagnostic::error_without_span(
                code::EXPORT_KEY_RESERVED,
                format!(
                    "{} has the property `{key}`: keys that start with `@` cannot be written",
                    kind.label(id)
                ),
            ));
            continue;
        }
        match value_text(value) {
            Ok(text) => parts.push(format!("{}: {}", names::name(key), text)),
            Err(why) => errors.push(Diagnostic::error_without_span(
                code::EXPORT_VALUE,
                format!("{}, property `{key}`: {why}", kind.label(id)),
            )),
        }
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!("{{{}}}", parts.join(", "))
    }
}

/// The text of an identity value (a string, or an integer in decimal), or why it is unusable.
fn identity_text(v: &Value) -> Result<String, &'static str> {
    match v {
        Value::String(s) if s.is_empty() => Err("the value is an empty string"),
        Value::String(s) => Ok(s.to_string()),
        Value::Int(i) => Ok(i.to_string()),
        _ => Err("the value is not a string or an integer"),
    }
}

/// The identifier an element takes from its stored `@id` or a configured
/// property, or `None` if it is generated (ADR-002 sections 2.1 and 2.2).
fn derive_identity(
    kind: Kind,
    id: i64,
    props: &BTreeMap<String, Value>,
    mode: IdentityMode,
    property: Option<&str>,
    errors: &mut Vec<Diagnostic>,
) -> Option<String> {
    if mode == IdentityMode::Generate {
        return None;
    }
    if mode == IdentityMode::Auto {
        if let Some(v) = props.get("@id") {
            return match v {
                Value::String(s) if !s.is_empty() => Some(s.to_string()),
                _ => {
                    errors.push(Diagnostic::error_without_span(
                        code::EXPORT_ID_UNUSABLE,
                        format!(
                            "{} has an `@id` that is not a non-empty string",
                            kind.label(id)
                        ),
                    ));
                    None
                }
            };
        }
    }
    let property = property?;
    match props.get(property) {
        None => {
            errors.push(Diagnostic::error_without_span(
                code::EXPORT_ID_MISSING,
                format!(
                    "{} has no `{property}` property to use as its identifier",
                    kind.label(id)
                ),
            ));
            None
        }
        Some(v) => match identity_text(v) {
            Ok(t) => Some(t),
            Err(why) => {
                errors.push(Diagnostic::error_without_span(
                    code::EXPORT_ID_UNUSABLE,
                    format!(
                        "{}: `{property}` cannot be an identifier: {why}",
                        kind.label(id)
                    ),
                ));
                None
            }
        },
    }
}

/// A failed export. No text was produced.
#[derive(Clone, Debug, thiserror::Error)]
#[error("the export failed with {total} error(s)")]
pub struct ExportFailure {
    pub diagnostics: Vec<Diagnostic>,
    pub total: usize,
}

impl ExportFailure {
    fn new(diagnostics: Vec<Diagnostic>) -> Self {
        let total = diagnostics.len();
        ExportFailure { diagnostics, total }
    }
}

/// Export the database behind `conn` as gram text (contracts/library-api.md).
pub fn export(conn: &mut Connection, settings: &ExportSettings) -> Result<String, ExportFailure> {
    if conn.in_transaction() {
        return Err(ExportFailure::new(vec![Diagnostic::error_without_span(
            code::EXPORT_IN_TRANSACTION,
            "the connection is already in a transaction; commit or roll it back first",
        )]));
    }
    let mut errors = Vec::new();
    match settings.identity {
        IdentityMode::Property
            if settings.node_id_property.is_none()
                && settings.relationship_id_property.is_none() =>
        {
            errors.push(Diagnostic::error_without_span(
                code::EXPORT_CONFIG,
                "the `property` identity mode needs `node_id_property` or `relationship_id_property`",
            ));
        }
        IdentityMode::Generate
            if settings.node_id_property.is_some()
                || settings.relationship_id_property.is_some() =>
        {
            errors.push(Diagnostic::error_without_span(
                code::EXPORT_CONFIG,
                "an identity property is set, but the `generate` mode ignores it",
            ));
        }
        _ => {}
    }
    if !errors.is_empty() {
        return Err(ExportFailure::new(errors));
    }

    // One transaction, so nodes and relationships come from one state.
    conn.begin()
        .map_err(|e| ExportFailure::new(vec![db_error(e)]))?;
    let graph = read_graph(conn);
    let _ = conn.rollback();
    let (nodes, rels) = graph.map_err(|e| ExportFailure::new(vec![e]))?;

    // Records first (they find every value problem), then identifiers.
    let node_records: Vec<String> = nodes
        .iter()
        .map(|n| record_text(Kind::Node, n.id, &n.props, &mut errors))
        .collect();
    let rel_records: Vec<String> = rels
        .iter()
        .map(|r| record_text(Kind::Relationship, r.id, &r.props, &mut errors))
        .collect();

    let node_ident: Vec<Option<String>> = nodes
        .iter()
        .map(|n| {
            derive_identity(
                Kind::Node,
                n.id,
                &n.props,
                settings.identity,
                settings.node_id_property.as_deref(),
                &mut errors,
            )
        })
        .collect();
    let rel_ident: Vec<Option<String>> = rels
        .iter()
        .map(|r| {
            derive_identity(
                Kind::Relationship,
                r.id,
                &r.props,
                settings.identity,
                settings.relationship_id_property.as_deref(),
                &mut errors,
            )
        })
        .collect();

    // Uniqueness across nodes and relationships together (ADR-002 section 2.2).
    let mut owners: HashMap<&str, Vec<String>> = HashMap::new();
    for (n, t) in nodes.iter().zip(&node_ident) {
        if let Some(t) = t {
            owners.entry(t).or_default().push(Kind::Node.label(n.id));
        }
    }
    for (r, t) in rels.iter().zip(&rel_ident) {
        if let Some(t) = t {
            owners
                .entry(t)
                .or_default()
                .push(Kind::Relationship.label(r.id));
        }
    }
    let mut dup_keys: Vec<&&str> = owners
        .iter()
        .filter(|(_, v)| v.len() > 1)
        .map(|(k, _)| k)
        .collect();
    dup_keys.sort();
    for key in dup_keys {
        let all = &owners[*key];
        for who in all {
            errors.push(Diagnostic::error_without_span(
                code::EXPORT_ID_DUPLICATE,
                format!(
                    "{who} has the identifier `{key}`, which is also used by {}",
                    all.iter()
                        .filter(|w| *w != who)
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            ));
        }
    }
    if !errors.is_empty() {
        return Err(ExportFailure::new(errors));
    }

    // Generated identifiers: n0, n1, ... and r0, r1, ..., skipping any in use.
    let mut used: HashSet<String> = node_ident
        .iter()
        .chain(&rel_ident)
        .flatten()
        .cloned()
        .collect();
    let mut next = |prefix: &str, counter: &mut usize| loop {
        let candidate = format!("{prefix}{counter}");
        *counter += 1;
        if used.insert(candidate.clone()) {
            return candidate;
        }
    };
    let (mut nc, mut rc) = (0usize, 0usize);
    let node_names: Vec<String> = node_ident
        .into_iter()
        .map(|t| t.unwrap_or_else(|| next("n", &mut nc)))
        .collect();
    let rel_names: Vec<String> = rel_ident
        .into_iter()
        .map(|t| t.unwrap_or_else(|| next("r", &mut rc)))
        .collect();
    let by_id: HashMap<i64, &str> = nodes
        .iter()
        .zip(&node_names)
        .map(|(n, t)| (n.id, t.as_str()))
        .collect();

    let mut out = String::new();
    for ((n, ident), record) in nodes.iter().zip(&node_names).zip(&node_records) {
        out.push('(');
        out.push_str(&names::identifier(ident));
        for l in &n.labels {
            out.push(':');
            out.push_str(&names::name(l));
        }
        if !record.is_empty() {
            out.push(' ');
            out.push_str(record);
        }
        out.push_str(")\n");
    }
    if !nodes.is_empty() && !rels.is_empty() {
        out.push('\n');
    }
    for ((r, ident), record) in rels.iter().zip(&rel_names).zip(&rel_records) {
        let start = by_id.get(&r.start).copied().unwrap_or_default();
        let end = by_id.get(&r.end).copied().unwrap_or_default();
        out.push_str(&format!(
            "({})-[{}:{}",
            names::identifier(start),
            names::identifier(ident),
            names::name(&r.ty)
        ));
        if !record.is_empty() {
            out.push(' ');
            out.push_str(record);
        }
        out.push_str(&format!("]->({})\n", names::identifier(end)));
    }
    Ok(out)
}
