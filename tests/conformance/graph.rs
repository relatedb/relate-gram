//! Graphs as sorted lines (ADR-015 section 2.3): a node is its sorted labels and its properties by
//! sorted key, with each value as gram-value canonical text; a relationship is its type, its
//! properties and the lines of its end nodes. Two graphs are equal when the sorted line lists are
//! equal, which depends on no internal identifier and on no order.
use super::value;
use gram_value::Value;
use relate::{Connection, Value as RV};
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Graph {
    pub nodes: Vec<String>,
    pub relationships: Vec<String>,
}

impl Graph {
    pub fn new(mut nodes: Vec<String>, mut relationships: Vec<String>) -> Graph {
        nodes.sort();
        relationships.sort();
        Graph {
            nodes,
            relationships,
        }
    }
}

fn key_text(k: &str) -> String {
    let plain = !k.is_empty()
        && k.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '@' | '-'))
        && !k.chars().next().unwrap().is_ascii_digit();
    if plain {
        k.to_string()
    } else {
        Value::string(k)
            .to_text()
            .unwrap_or_else(|_| format!("{k:?}"))
    }
}

pub fn props_text(props: &BTreeMap<String, Value>) -> Result<String, String> {
    let mut parts = Vec::new();
    for (k, v) in props {
        let text = v.to_text().map_err(|ds| {
            ds.into_iter()
                .map(|d| d.message)
                .collect::<Vec<_>>()
                .join("; ")
        })?;
        parts.push(format!("{}: {text}", key_text(k)));
    }
    Ok(format!("{{{}}}", parts.join(", ")))
}

pub fn node_line(labels: &[String], props: &BTreeMap<String, Value>) -> Result<String, String> {
    let mut labels = labels.to_vec();
    labels.sort();
    let l: String = labels.iter().map(|x| format!(":{x}")).collect();
    let sep = if l.is_empty() { "" } else { " " };
    Ok(format!("{l}{sep}{}", props_text(props)?))
}

pub fn relationship_line(
    ty: &str,
    props: &BTreeMap<String, Value>,
    start: &str,
    end: &str,
) -> Result<String, String> {
    Ok(format!(
        "({start}) -[{ty} {}]-> ({end})",
        props_text(props)?
    ))
}

fn is_generated(id: &str) -> bool {
    let mut cs = id.chars();
    matches!(cs.next(), Some('n') | Some('r')) && id.len() > 1 && cs.all(|c| c.is_ascii_digit())
}

fn text_of(v: &RV) -> Result<String, String> {
    match v {
        RV::String(s) => Ok(s.to_string()),
        other => Err(format!("expected text, found {other:?}")),
    }
}

fn float_of(v: &RV) -> Result<f64, String> {
    match v {
        RV::Float(f) => Ok(*f),
        RV::Int(i) => Ok(*i as f64),
        other => Err(format!("expected a number, found {other:?}")),
    }
}

fn params(pairs: Vec<(&str, RV)>) -> BTreeMap<String, RV> {
    pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
}

/// The properties of the node or relationship with internal id `id` as gram values.
pub fn properties(
    conn: &mut Connection,
    entity: &str,
    id: i64,
    strip_generated: bool,
) -> Result<BTreeMap<String, Value>, String> {
    let keys_q = format!("MATCH {entity} WHERE id(e) = $id RETURN keys(e)");
    let rows = conn
        .query_with_params(&keys_q, params(vec![("id", RV::Int(id))]))
        .map_err(|e| e.to_string())?
        .rows;
    let keys: Vec<String> = match rows.first().and_then(|r| r.first()) {
        Some(RV::List(items)) => items.iter().map(text_of).collect::<Result<_, _>>()?,
        _ => Vec::new(),
    };
    let mut out = BTreeMap::new();
    for k in keys {
        let p = |extra: Vec<(&'static str, RV)>| {
            let mut v = vec![
                ("id", RV::Int(id)),
                ("k", RV::String(Arc::from(k.as_str()))),
            ];
            v.extend(extra);
            params(v)
        };
        let q =
            format!("MATCH {entity} WHERE id(e) = $id RETURN valueType(e[$k]), toString(e[$k])");
        let r = conn
            .query_with_params(&q, p(vec![]))
            .map_err(|e| e.to_string())?
            .rows;
        let (vt, text) = (text_of(&r[0][0])?, text_of(&r[0][1])?);
        let t = vt.strip_suffix(" NOT NULL").unwrap_or(&vt).to_string();
        let value = if t.starts_with("LIST<POINT") {
            let q = format!(
                "MATCH {entity} WHERE id(e) = $id RETURN [x IN e[$k] | [x.srid, x.x, x.y, \
                 CASE WHEN x.srid IN [4979, 9157] THEN x.z ELSE null END]]"
            );
            let r = conn
                .query_with_params(&q, p(vec![]))
                .map_err(|e| e.to_string())?
                .rows;
            let mut items = Vec::new();
            match &r[0][0] {
                RV::List(points) => {
                    for pt in points.iter() {
                        let RV::List(f) = pt else {
                            return Err(format!("expected a point, found {pt:?}"));
                        };
                        let srid = match &f[0] {
                            RV::Int(i) => *i,
                            other => return Err(format!("expected an SRID, found {other:?}")),
                        };
                        let z = match &f[3] {
                            RV::Null => None,
                            other => Some(float_of(other)?),
                        };
                        items.push(value::database_point(
                            srid,
                            float_of(&f[1])?,
                            float_of(&f[2])?,
                            z,
                        )?);
                    }
                }
                other => return Err(format!("expected a list, found {other:?}")),
            }
            Value::array(items)
        } else if t.starts_with("LIST<") {
            let q = format!("MATCH {entity} WHERE id(e) = $id RETURN [x IN e[$k] | toString(x)]");
            let r = conn
                .query_with_params(&q, p(vec![]))
                .map_err(|e| e.to_string())?
                .rows;
            let items: Vec<String> = match &r[0][0] {
                RV::List(items) => items.iter().map(text_of).collect::<Result<_, _>>()?,
                other => return Err(format!("expected a list, found {other:?}")),
            };
            value::database_list(&vt, &items)?
        } else if t == "POINT" {
            let q = format!(
                "MATCH {entity} WHERE id(e) = $id RETURN e[$k].srid, e[$k].x, e[$k].y, \
                 CASE WHEN e[$k].srid IN [4979, 9157] THEN e[$k].z ELSE null END"
            );
            let r = conn
                .query_with_params(&q, p(vec![]))
                .map_err(|e| e.to_string())?
                .rows;
            let srid = match &r[0][0] {
                RV::Int(i) => *i,
                other => return Err(format!("expected an SRID, found {other:?}")),
            };
            let z = match &r[0][3] {
                RV::Null => None,
                other => Some(float_of(other)?),
            };
            value::database_point(srid, float_of(&r[0][1])?, float_of(&r[0][2])?, z)?
        } else {
            value::database_scalar(&vt, &text)?
        };
        if strip_generated && k == "@id" {
            if let RV::String(_) = &r[0][1] {
                // a generated identifier is compared without its `@id`
            }
            if is_generated(&text) && t == "STRING" {
                continue;
            }
        }
        out.insert(k, value);
    }
    Ok(out)
}

/// Read the whole database as lines. With `strip_generated`, an `@id` of the form `n0` or `r0` is
/// left out, so a reload of an export equals the original (ADR-015 section 2.3).
pub fn dump_database(conn: &mut Connection, strip_generated: bool) -> Result<Graph, String> {
    let rows = conn
        .query("MATCH (e) RETURN id(e), labels(e) ORDER BY id(e)")
        .map_err(|e| e.to_string())?
        .rows;
    let mut lines: BTreeMap<i64, String> = BTreeMap::new();
    for row in &rows {
        let id = match &row[0] {
            RV::Int(i) => *i,
            other => return Err(format!("expected an id, found {other:?}")),
        };
        let labels: Vec<String> = match &row[1] {
            RV::List(l) => l.iter().map(text_of).collect::<Result<_, _>>()?,
            other => return Err(format!("expected labels, found {other:?}")),
        };
        let props = properties(conn, "(e)", id, strip_generated)?;
        lines.insert(id, node_line(&labels, &props)?);
    }
    let rows = conn
        .query("MATCH (a)-[e]->(b) RETURN id(e), type(e), id(a), id(b) ORDER BY id(e)")
        .map_err(|e| e.to_string())?
        .rows;
    let mut rels = Vec::new();
    for row in &rows {
        let (id, a, b) = match (&row[0], &row[2], &row[3]) {
            (RV::Int(i), RV::Int(a), RV::Int(b)) => (*i, *a, *b),
            other => return Err(format!("expected ids, found {other:?}")),
        };
        let ty = text_of(&row[1])?;
        let props = properties(conn, "()-[e]->()", id, strip_generated)?;
        rels.push(relationship_line(&ty, &props, &lines[&a], &lines[&b])?);
    }
    Ok(Graph::new(lines.into_values().collect(), rels))
}

fn only_in(a: &[String], b: &[String]) -> Vec<String> {
    let mut rest: Vec<&String> = b.iter().collect();
    let mut out = Vec::new();
    for x in a {
        if let Some(i) = rest.iter().position(|y| *y == x) {
            rest.remove(i);
        } else {
            out.push(x.clone());
        }
    }
    out
}

/// What differs between an expected graph and the one read from the database, as a message.
pub fn describe_difference(expected: &Graph, got: &Graph) -> String {
    let mut parts = Vec::new();
    for l in only_in(&expected.nodes, &got.nodes) {
        parts.push(format!("expected node missing: {l}"));
    }
    for l in only_in(&got.nodes, &expected.nodes) {
        parts.push(format!("unexpected node: {l}"));
    }
    for l in only_in(&expected.relationships, &got.relationships) {
        parts.push(format!("expected relationship missing: {l}"));
    }
    for l in only_in(&got.relationships, &expected.relationships) {
        parts.push(format!("unexpected relationship: {l}"));
    }
    parts.join("; ")
}
