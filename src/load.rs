//! Matching a load plan against the database and writing it in one
//! transaction (ADR-001 section 2.7, research R4 to R6).

use crate::diagnostics::{code, Diagnostic};
use crate::names::cypher_name;
use crate::plan::{node_identity, rel_identity, LoadPlan, PlannedNode, PlannedRel};
use crate::settings::LoadSettings;
use crate::values::PropValue;
use relate::{Connection, Value};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

/// The most rows one statement carries (research R5).
const BATCH: usize = 5_000;

#[derive(Debug, Default)]
pub(crate) struct Written {
    pub nodes_created: usize,
    pub nodes_updated: usize,
    pub relationships_created: usize,
    pub relationships_updated: usize,
    /// Warnings that depend on what the database held.
    pub warnings: Vec<Diagnostic>,
}

fn db_error(e: impl std::fmt::Display) -> Diagnostic {
    Diagnostic::error_without_span(code::LOAD_DATABASE_ERROR, e.to_string())
}

fn to_value(v: &PropValue) -> Value {
    match v {
        PropValue::Int(i) => Value::Int(*i),
        PropValue::Float(f) => Value::Float(*f),
        PropValue::Bool(b) => Value::Bool(*b),
        PropValue::Str(s) => Value::String(Arc::from(s.as_str())),
        PropValue::List(items) => Value::List(Arc::new(items.iter().map(to_value).collect())),
    }
}

fn map_value(map: BTreeMap<String, Value>) -> Value {
    Value::Map(Arc::new(map))
}

/// The properties to write for an element. `identity` is the gram identifier to
/// store as `@id`, or `None` when identity comes from a property.
fn props_value(identity: Option<&str>, props: &[(String, PropValue)]) -> Value {
    let mut map: BTreeMap<String, Value> = BTreeMap::new();
    if let Some(id) = identity {
        map.insert("@id".to_string(), Value::String(Arc::from(id)));
    }
    for (k, v) in props {
        map.insert(k.clone(), to_value(v));
    }
    map_value(map)
}

fn int_value(v: &Value) -> Option<i64> {
    match v {
        Value::Int(i) => Some(*i),
        _ => None,
    }
}

/// The text of a stored identity value (ADR-001 section 2.1).
fn stored_text(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.to_string()),
        Value::Int(i) => Some(i.to_string()),
        _ => None,
    }
}

/// The sorted, de-duplicated label set that groups nodes.
fn label_set(n: &PlannedNode) -> Vec<String> {
    let mut l = n.labels.clone();
    l.sort();
    l.dedup();
    l
}

fn label_pattern(var: &str, set: &[String]) -> Result<String, Diagnostic> {
    let mut pattern = String::from(var);
    for l in set {
        pattern.push(':');
        pattern.push_str(&cypher_name(l).map_err(|_| db_error("a label contains a NUL"))?);
    }
    Ok(pattern)
}

fn batch_params(rows: Vec<Value>) -> BTreeMap<String, Value> {
    let mut params = BTreeMap::new();
    params.insert("rows".to_string(), Value::List(Arc::new(rows)));
    params
}

struct StoredRel {
    id: i64,
    ty: String,
    start: i64,
    end: i64,
}

/// One scan of the existing elements, as a table from identity text (ADR-001
/// section 2.7; SPIKE-001 section 4.6).
#[derive(Default)]
struct IdentityTable {
    nodes: HashMap<String, Vec<i64>>,
    rels: HashMap<String, Vec<StoredRel>>,
}

fn scan(conn: &mut Connection, settings: &LoadSettings) -> Result<IdentityTable, Diagnostic> {
    let node_prop = settings.node_id_property.as_deref().unwrap_or("@id");
    let rel_prop = settings
        .relationship_id_property
        .as_deref()
        .unwrap_or("@id");
    let node_q = format!(
        "MATCH (n) RETURN id(n), n.{}",
        cypher_name(node_prop).map_err(|_| db_error("the identity property contains a NUL"))?
    );
    let rel_q = format!(
        "MATCH (a)-[r]->(b) RETURN id(r), r.{}, type(r), id(a), id(b)",
        cypher_name(rel_prop).map_err(|_| db_error("the identity property contains a NUL"))?
    );
    let mut table = IdentityTable::default();
    for row in conn.query(&node_q).map_err(db_error)?.rows {
        if let (Some(id), Some(text)) = (int_value(&row[0]), stored_text(&row[1])) {
            table.nodes.entry(text).or_default().push(id);
        }
    }
    for row in conn.query(&rel_q).map_err(db_error)?.rows {
        let Some(text) = stored_text(&row[1]) else {
            continue;
        };
        let (Some(id), Some(start), Some(end)) =
            (int_value(&row[0]), int_value(&row[3]), int_value(&row[4]))
        else {
            continue;
        };
        let ty = match &row[2] {
            Value::String(s) => s.to_string(),
            _ => continue,
        };
        table
            .rels
            .entry(text)
            .or_default()
            .push(StoredRel { id, ty, start, end });
    }
    Ok(table)
}

/// For each planned element, the stored element it matches, if any.
struct Resolution {
    node: Vec<Option<i64>>,
    rel: Vec<Option<i64>>,
}

fn resolve(
    plan: &LoadPlan,
    table: &IdentityTable,
    settings: &LoadSettings,
) -> Result<Resolution, Vec<Diagnostic>> {
    let mut errors = Vec::new();
    // Nodes and relationships share the `@id` space only when both use it.
    let shared = settings.node_id_property.is_none() && settings.relationship_id_property.is_none();
    let mut node: Vec<Option<i64>> = Vec::with_capacity(plan.nodes.len());
    for n in &plan.nodes {
        let Some(text) = node_identity(n, settings) else {
            node.push(None);
            continue;
        };
        match table.nodes.get(&text).map(Vec::as_slice) {
            None => {
                if shared && table.rels.contains_key(&text) {
                    errors.push(Diagnostic::error(
                        code::MATCH_KIND_DIFFERS,
                        n.span,
                        format!(
                            "`{text}` is a stored relationship, and the file uses it as a node"
                        ),
                    ));
                }
                node.push(None);
            }
            Some([one]) => node.push(Some(*one)),
            Some(many) => {
                errors.push(Diagnostic::error(
                    code::MATCH_DB_DUPLICATE,
                    n.span,
                    format!(
                        "{} stored nodes have the identity `{text}`: ids {}",
                        many.len(),
                        join_ids(many)
                    ),
                ));
                node.push(None);
            }
        }
    }
    let mut rel: Vec<Option<i64>> = Vec::with_capacity(plan.rels.len());
    for r in &plan.rels {
        let Some(text) = rel_identity(r, settings) else {
            rel.push(None);
            continue;
        };
        match table.rels.get(&text).map(Vec::as_slice) {
            None => {
                if shared && table.nodes.contains_key(&text) {
                    errors.push(Diagnostic::error(
                        code::MATCH_KIND_DIFFERS,
                        r.span,
                        format!(
                            "`{text}` is a stored node, and the file uses it as a relationship"
                        ),
                    ));
                }
                rel.push(None);
            }
            Some([one]) => {
                let ty = r.rel_type.as_deref().unwrap_or_default();
                let ends = (node[r.start], node[r.end]);
                if one.ty != ty || ends != (Some(one.start), Some(one.end)) {
                    errors.push(Diagnostic::error(
                        code::MATCH_TYPE_DIFFERS,
                        r.span,
                        format!(
                            "the stored relationship `{text}` is `{}` from node {} to node {}; \
                             the file has `{ty}` between different nodes or a different type",
                            one.ty, one.start, one.end
                        ),
                    ));
                }
                rel.push(Some(one.id));
            }
            Some(many) => {
                let ids: Vec<i64> = many.iter().map(|m| m.id).collect();
                errors.push(Diagnostic::error(
                    code::MATCH_DB_DUPLICATE,
                    r.span,
                    format!(
                        "{} stored relationships have the identity `{text}`: ids {}",
                        many.len(),
                        join_ids(&ids)
                    ),
                ));
                rel.push(None);
            }
        }
    }
    if errors.is_empty() {
        Ok(Resolution { node, rel })
    } else {
        Err(errors)
    }
}

fn join_ids(ids: &[i64]) -> String {
    ids.iter()
        .map(i64::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

/// Match the plan against the database and write it. On any failure the
/// transaction is rolled back and the errors returned; on success it is committed.
pub(crate) fn write(
    conn: &mut Connection,
    plan: &LoadPlan,
    settings: &LoadSettings,
) -> Result<Written, Vec<Diagnostic>> {
    conn.begin().map_err(|e| vec![db_error(e)])?;
    match write_in_transaction(conn, plan, settings) {
        Ok(w) => {
            conn.commit().map_err(|e| vec![db_error(e)])?;
            Ok(w)
        }
        Err(e) => {
            let _ = conn.rollback();
            Err(e)
        }
    }
}

fn write_in_transaction(
    conn: &mut Connection,
    plan: &LoadPlan,
    settings: &LoadSettings,
) -> Result<Written, Vec<Diagnostic>> {
    let table = scan(conn, settings).map_err(|e| vec![e])?;
    let res = resolve(plan, &table, settings)?;
    write_plan(conn, plan, settings, &res).map_err(|e| vec![e])
}

fn node_stored_id<'a>(n: &'a PlannedNode, settings: &LoadSettings) -> Option<&'a str> {
    // With an identity property the gram identifier is not stored (ADR-001 section 2.10).
    if settings.node_id_property.is_some() {
        None
    } else {
        n.key.as_deref()
    }
}

fn rel_stored_id<'a>(r: &'a PlannedRel, settings: &LoadSettings) -> Option<&'a str> {
    if settings.relationship_id_property.is_some() {
        None
    } else {
        r.key.as_deref()
    }
}

fn write_plan(
    conn: &mut Connection,
    plan: &LoadPlan,
    settings: &LoadSettings,
    res: &Resolution,
) -> Result<Written, Diagnostic> {
    let mut written = Written::default();
    let mut ids: Vec<i64> = res.node.iter().map(|m| m.unwrap_or(0)).collect();

    // Create the unmatched nodes: runs of consecutive nodes with one label set,
    // in order of first appearance.
    let to_create: Vec<usize> = (0..plan.nodes.len())
        .filter(|i| res.node[*i].is_none())
        .collect();
    let mut i = 0;
    while i < to_create.len() {
        let set = label_set(&plan.nodes[to_create[i]]);
        let mut j = i;
        while j < to_create.len() && j - i < BATCH && label_set(&plan.nodes[to_create[j]]) == set {
            j += 1;
        }
        let pattern = label_pattern("n", &set)?;
        let rows: Vec<Value> = to_create[i..j]
            .iter()
            .map(|&k| {
                let n = &plan.nodes[k];
                let mut row = BTreeMap::new();
                row.insert("i".to_string(), Value::Int(k as i64));
                row.insert(
                    "props".to_string(),
                    props_value(node_stored_id(n, settings), &n.props),
                );
                map_value(row)
            })
            .collect();
        let cypher =
            format!("UNWIND $rows AS r CREATE ({pattern}) SET n = r.props RETURN r.i, id(n)");
        let result = conn
            .query_with_params(&cypher, batch_params(rows))
            .map_err(db_error)?;
        if result.rows.len() != j - i {
            return Err(db_error(
                "a node batch returned an unexpected number of rows",
            ));
        }
        for row in &result.rows {
            let (Some(k), Some(id)) = (int_value(&row[0]), int_value(&row[1])) else {
                return Err(db_error("a node batch returned a non-integer id"));
            };
            ids[k as usize] = id;
        }
        written.nodes_created += j - i;
        i = j;
    }

    // Update the matched nodes the file defines: the file's properties win,
    // stated labels are added, nothing else changes. A matched reference is left alone.
    let matched_defined: Vec<usize> = (0..plan.nodes.len())
        .filter(|&i| res.node[i].is_some() && plan.nodes[i].defined)
        .collect();
    for chunk in matched_defined.chunks(BATCH) {
        let rows: Vec<Value> = chunk
            .iter()
            .map(|&k| {
                let n = &plan.nodes[k];
                let mut row = BTreeMap::new();
                row.insert("id".to_string(), Value::Int(ids[k]));
                row.insert("props".to_string(), props_value(None, &n.props));
                map_value(row)
            })
            .collect();
        conn.query_with_params(
            "UNWIND $rows AS r MATCH (n) WHERE id(n) = r.id SET n += r.props",
            batch_params(rows),
        )
        .map_err(db_error)?;
    }
    let mut by_labels: BTreeMap<Vec<String>, Vec<Value>> = BTreeMap::new();
    for &k in &matched_defined {
        let set = label_set(&plan.nodes[k]);
        if !set.is_empty() {
            let mut row = BTreeMap::new();
            row.insert("id".to_string(), Value::Int(ids[k]));
            by_labels.entry(set).or_default().push(map_value(row));
        }
    }
    for (set, rows) in by_labels {
        let pattern = label_pattern("n", &set)?;
        for chunk in rows.chunks(BATCH) {
            conn.query_with_params(
                &format!("UNWIND $rows AS r MATCH (n) WHERE id(n) = r.id SET {pattern}"),
                batch_params(chunk.to_vec()),
            )
            .map_err(db_error)?;
        }
    }
    written.nodes_updated = matched_defined.len();

    // Update the matched relationships.
    let matched_rels: Vec<usize> = (0..plan.rels.len())
        .filter(|&i| res.rel[i].is_some())
        .collect();
    for chunk in matched_rels.chunks(BATCH) {
        let rows: Vec<Value> = chunk
            .iter()
            .map(|&k| {
                let r = &plan.rels[k];
                let mut row = BTreeMap::new();
                row.insert("id".to_string(), Value::Int(res.rel[k].unwrap_or_default()));
                row.insert("props".to_string(), props_value(None, &r.props));
                map_value(row)
            })
            .collect();
        conn.query_with_params(
            "UNWIND $rows AS r MATCH ()-[x]->() WHERE id(x) = r.id SET x += r.props",
            batch_params(rows),
        )
        .map_err(db_error)?;
    }
    written.relationships_updated = matched_rels.len();

    // Create the unmatched relationships: runs with one type, in order of appearance.
    let rel_create: Vec<usize> = (0..plan.rels.len())
        .filter(|i| res.rel[*i].is_none())
        .collect();
    let ty_of = |k: usize| plan.rels[k].rel_type.as_deref().unwrap_or_default();
    let mut i = 0;
    while i < rel_create.len() {
        let ty = ty_of(rel_create[i]);
        let mut j = i;
        while j < rel_create.len() && j - i < BATCH && ty_of(rel_create[j]) == ty {
            j += 1;
        }
        let name = cypher_name(ty).map_err(|_| db_error("a type contains a NUL"))?;
        let rows: Vec<Value> = rel_create[i..j]
            .iter()
            .map(|&k| {
                let r = &plan.rels[k];
                let mut row = BTreeMap::new();
                row.insert("s".to_string(), Value::Int(ids[r.start]));
                row.insert("e".to_string(), Value::Int(ids[r.end]));
                row.insert(
                    "props".to_string(),
                    props_value(rel_stored_id(r, settings), &r.props),
                );
                map_value(row)
            })
            .collect();
        let cypher = format!(
            "UNWIND $rows AS r MATCH (a) WHERE id(a) = r.s MATCH (b) WHERE id(b) = r.e \
             CREATE (a)-[x:{name}]->(b) SET x = r.props"
        );
        conn.query_with_params(&cypher, batch_params(rows))
            .map_err(db_error)?;
        written.relationships_created += j - i;
        i = j;
    }

    // Nodes created only because they were referenced (ADR-001 section 2.3).
    for (k, n) in plan.nodes.iter().enumerate() {
        if let (Some(key), false, None) = (&n.key, n.defined, res.node[k]) {
            written.warnings.push(Diagnostic::warning(
                code::ID_REFERENCE_ONLY,
                n.span,
                format!("`{key}` is referenced but never defined; an empty node was created"),
            ));
        }
    }
    Ok(written)
}
