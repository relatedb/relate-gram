//! Shared test support: database setup, diagnostic assertions and graph comparison.
#![allow(dead_code)]

use relate::{Connection, Database, Value};
use relate_gram::diagnostics::Diagnostic;
use std::collections::BTreeMap;

/// A fresh in-memory database and a connection to it. The database is
/// returned too so it outlives the connection.
pub fn fresh_db() -> (Database, Connection) {
    let db = Database::open(":memory:").expect("open an in-memory database");
    let conn = db.connect().expect("connect");
    (db, conn)
}

/// Run a Cypher statement, panicking on failure.
pub fn run(conn: &mut Connection, cypher: &str) -> Vec<Vec<Value>> {
    conn.query(cypher)
        .unwrap_or_else(|e| panic!("`{cypher}` failed: {e}"))
        .rows
}

/// Assert that `diags` holds an error with `code` at `line`:`column`.
pub fn assert_diag(diags: &[Diagnostic], code: &str, line: usize, column: usize) {
    let found: Vec<(&str, Option<(usize, usize)>)> = diags
        .iter()
        .map(|d| (d.code, d.span.map(|s| (s.line, s.column))))
        .collect();
    assert!(
        found
            .iter()
            .any(|(c, p)| *c == code && *p == Some((line, column))),
        "expected {code} at {line}:{column}, got {found:?}"
    );
}

pub fn codes(diags: &[Diagnostic]) -> Vec<&'static str> {
    diags.iter().map(|d| d.code).collect()
}

/// Render a value as stable text for comparisons.
pub fn fmt_value(v: &Value) -> String {
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Int(i) => format!("{i}"),
        Value::Float(f) => format!("{f:?}"),
        Value::String(s) => format!("{s:?}"),
        Value::List(items) => {
            let inner: Vec<String> = items.iter().map(fmt_value).collect();
            format!("[{}]", inner.join(", "))
        }
        Value::Map(m) => fmt_map(m),
        other => format!("{other:?}"),
    }
}

fn fmt_map(m: &BTreeMap<String, Value>) -> String {
    let inner: Vec<String> = m
        .iter()
        .map(|(k, v)| format!("{k:?}: {}", fmt_value(v)))
        .collect();
    format!("{{{}}}", inner.join(", "))
}

/// The graph in a comparable form: every node and relationship as a line of
/// text, sorted. Nodes are `labels | properties`; relationships are
/// `(start) -[TYPE properties]-> (end)` where endpoints are written as the
/// endpoint's own node line, so the dump does not depend on internal ids.
#[derive(Debug, PartialEq, Eq)]
pub struct GraphDump {
    pub nodes: Vec<String>,
    pub relationships: Vec<String>,
}

pub fn dump_graph(conn: &mut Connection) -> GraphDump {
    let node_rows = run(
        conn,
        "MATCH (n) RETURN id(n), labels(n), properties(n) ORDER BY id(n)",
    );
    let mut by_id: BTreeMap<i64, String> = BTreeMap::new();
    let mut nodes = Vec::new();
    for row in &node_rows {
        let id = match &row[0] {
            Value::Int(i) => *i,
            other => panic!("id() was {other:?}"),
        };
        let mut labels: Vec<String> = match &row[1] {
            Value::List(l) => l
                .iter()
                .map(|v| match v {
                    Value::String(s) => s.to_string(),
                    o => panic!("label {o:?}"),
                })
                .collect(),
            o => panic!("labels() was {o:?}"),
        };
        labels.sort();
        let props = match &row[2] {
            Value::Map(m) => fmt_map(m),
            o => panic!("properties() was {o:?}"),
        };
        let line = format!(":{} {}", labels.join(":"), props);
        by_id.insert(id, line.clone());
        nodes.push(line);
    }
    nodes.sort();
    let rel_rows = run(
        conn,
        "MATCH (a)-[r]->(b) RETURN id(a), type(r), properties(r), id(b) ORDER BY id(r)",
    );
    let mut relationships = Vec::new();
    for row in &rel_rows {
        let (a, b) = match (&row[0], &row[3]) {
            (Value::Int(a), Value::Int(b)) => (*a, *b),
            o => panic!("endpoint ids {o:?}"),
        };
        let ty = match &row[1] {
            Value::String(s) => s.to_string(),
            o => panic!("type {o:?}"),
        };
        let props = match &row[2] {
            Value::Map(m) => fmt_map(m),
            o => panic!("properties {o:?}"),
        };
        relationships.push(format!(
            "({}) -[{} {}]-> ({})",
            by_id[&a], ty, props, by_id[&b]
        ));
    }
    relationships.sort();
    GraphDump {
        nodes,
        relationships,
    }
}
