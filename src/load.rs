//! Writing a load plan to the database in one transaction (ADR-001 section 2.7,
//! research R4 to R6).

use crate::diagnostics::{code, Diagnostic};
use crate::names::cypher_name;
use crate::plan::{LoadPlan, PlannedNode, PlannedRel};
use crate::values::PropValue;
use relate::{Connection, Value};
use std::collections::BTreeMap;
use std::sync::Arc;

/// The most rows one statement carries (research R5).
const BATCH: usize = 5_000;

#[derive(Debug, Default)]
pub(crate) struct Written {
    pub nodes_created: usize,
    pub relationships_created: usize,
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

fn props_value(identity: Option<&str>, props: &[(String, PropValue)]) -> Value {
    let mut map: BTreeMap<String, Value> = BTreeMap::new();
    if let Some(id) = identity {
        map.insert("@id".to_string(), Value::String(Arc::from(id)));
    }
    for (k, v) in props {
        map.insert(k.clone(), to_value(v));
    }
    Value::Map(Arc::new(map))
}

fn int_value(v: &Value) -> Option<i64> {
    match v {
        Value::Int(i) => Some(*i),
        _ => None,
    }
}

/// The sorted, de-duplicated label set that groups a run of nodes.
fn label_set(n: &PlannedNode) -> Vec<String> {
    let mut l = n.labels.clone();
    l.sort();
    l.dedup();
    l
}

/// Write the plan. On any failure the transaction is rolled back and the
/// error returned; on success it is committed.
pub(crate) fn write(conn: &mut Connection, plan: &LoadPlan) -> Result<Written, Diagnostic> {
    conn.begin().map_err(db_error)?;
    match write_in_transaction(conn, plan) {
        Ok(w) => {
            conn.commit().map_err(db_error)?;
            Ok(w)
        }
        Err(e) => {
            let _ = conn.rollback();
            Err(e)
        }
    }
}

fn write_in_transaction(conn: &mut Connection, plan: &LoadPlan) -> Result<Written, Diagnostic> {
    let mut ids: Vec<i64> = vec![0; plan.nodes.len()];
    let mut written = Written::default();

    // Nodes: runs of consecutive nodes with the same label set, in order of appearance.
    let mut i = 0;
    while i < plan.nodes.len() {
        let set = label_set(&plan.nodes[i]);
        let mut j = i;
        while j < plan.nodes.len() && j - i < BATCH && label_set(&plan.nodes[j]) == set {
            j += 1;
        }
        let mut pattern = String::from("n");
        for l in &set {
            pattern.push(':');
            pattern.push_str(&cypher_name(l).map_err(|_| {
                Diagnostic::error_without_span(code::LOAD_DATABASE_ERROR, "a label contains a NUL")
            })?);
        }
        let rows: Vec<Value> = (i..j)
            .map(|k| {
                let n = &plan.nodes[k];
                let mut row = BTreeMap::new();
                row.insert("i".to_string(), Value::Int(k as i64));
                row.insert("props".to_string(), props_value(n.key.as_deref(), &n.props));
                Value::Map(Arc::new(row))
            })
            .collect();
        let cypher =
            format!("UNWIND $rows AS r CREATE ({pattern}) SET n = r.props RETURN r.i, id(n)");
        let mut params = BTreeMap::new();
        params.insert("rows".to_string(), Value::List(Arc::new(rows)));
        let result = conn.query_with_params(&cypher, params).map_err(db_error)?;
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

    // Relationships: runs of consecutive relationships with the same type.
    let mut i = 0;
    while i < plan.rels.len() {
        let ty = &plan.rels[i].rel_type;
        let mut j = i;
        while j < plan.rels.len() && j - i < BATCH && &plan.rels[j].rel_type == ty {
            j += 1;
        }
        let name = cypher_name(ty).map_err(|_| {
            Diagnostic::error_without_span(code::LOAD_DATABASE_ERROR, "a type contains a NUL")
        })?;
        let rows: Vec<Value> = plan.rels[i..j].iter().map(|r| rel_row(r, &ids)).collect();
        let cypher = format!(
            "UNWIND $rows AS r MATCH (a) WHERE id(a) = r.s MATCH (b) WHERE id(b) = r.e \
             CREATE (a)-[x:{name}]->(b) SET x = r.props"
        );
        let mut params = BTreeMap::new();
        params.insert("rows".to_string(), Value::List(Arc::new(rows)));
        conn.query_with_params(&cypher, params).map_err(db_error)?;
        written.relationships_created += j - i;
        i = j;
    }
    Ok(written)
}

fn rel_row(r: &PlannedRel, ids: &[i64]) -> Value {
    let mut row = BTreeMap::new();
    row.insert("s".to_string(), Value::Int(ids[r.start]));
    row.insert("e".to_string(), Value::Int(ids[r.end]));
    row.insert("props".to_string(), props_value(r.key.as_deref(), &r.props));
    Value::Map(Arc::new(row))
}
