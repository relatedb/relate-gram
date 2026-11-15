//! ADR-014: export then load keeps float vectors exactly (Constitution IV, SC-001).
use super::common::*;
use relate::Value;
use std::collections::BTreeMap;
use std::sync::Arc;

fn floats(xs: &[f64]) -> Value {
    Value::List(Arc::new(xs.iter().map(|f| Value::Float(*f)).collect()))
}

fn create(c: &mut relate::Connection, kind: &str, xs: &[f64]) {
    let mut p = BTreeMap::new();
    p.insert("xs".to_string(), floats(xs));
    c.query_with_params(
        &format!("CREATE (:P {{e: vector($xs, size($xs), {kind})}})"),
        p,
    )
    .unwrap_or_else(|e| panic!("create {kind} dim {}: {e}", xs.len()));
}

fn read(c: &mut relate::Connection) -> (String, Vec<u64>) {
    let rows = run(c, "MATCH (n:P) RETURN n.e");
    let Value::Vector { coord, values } = &rows[0][0] else {
        panic!("expected a vector, got {:?}", rows[0][0]);
    };
    let bits = values
        .iter()
        .map(|v| match v {
            Value::Float(f) => f.to_bits(),
            other => panic!("{other:?}"),
        })
        .collect();
    (format!("{coord:?}"), bits)
}

fn series(n: usize, base: f64) -> Vec<f64> {
    (0..n)
        .map(|i| base + (i as f64) * 0.37 - (n as f64) * 0.1)
        .collect()
}

fn round_trip(kind: &str, xs: &[f64]) {
    let (_db, mut a) = fresh_db();
    create(&mut a, kind, xs);
    let before = read(&mut a);
    let text = export_str(&mut a).unwrap_or_else(|f| panic!("export {kind} {}: {f:?}", xs.len()));
    let (_db2, mut b) = fresh_db();
    load_str(&mut b, &text).unwrap_or_else(|f| panic!("reload {kind} {}: {f:?}", xs.len()));
    assert_eq!(read(&mut b), before, "{kind} dimension {}", xs.len());
}

#[test]
fn float32_vectors_of_each_dimension_round_trip() {
    for n in [1, 3, 384, 4096] {
        round_trip("FLOAT32", &series(n, 0.5));
    }
}

#[test]
fn float64_vectors_of_each_dimension_round_trip() {
    for n in [1, 3, 384, 4096] {
        round_trip("FLOAT64", &series(n, 0.25));
    }
}

#[test]
fn edge_elements_survive() {
    let f32_edges = [
        -0.0,
        f64::from(f32::MAX),
        f64::from(f32::MIN),
        f64::from(f32::MIN_POSITIVE),
        f64::from(f32::from_bits(1)),
    ];
    round_trip("FLOAT32", &f32_edges);
    let f64_edges = [
        -0.0,
        f64::MAX,
        f64::MIN,
        f64::MIN_POSITIVE,
        f64::from_bits(1),
    ];
    round_trip("FLOAT64", &f64_edges);
}
