//! SPIKE-002 probe. Disposable. Prints what Relate does with property values.
use relate::{Connection, Database, Value};
use std::collections::BTreeMap;
use std::sync::Arc;

fn show(r: &[Vec<Value>]) -> String {
    r.iter().map(|row| format!("{:?}", row)).collect::<Vec<_>>().join(" | ")
}
fn run(c: &mut Connection, q: &str) {
    match c.query(q) {
        Ok(r) => println!("  ok  {q}\n      -> {}", show(&r.rows)),
        Err(e) => println!("  ERR {q}\n      -> {e}"),
    }
}
fn runp(c: &mut Connection, label: &str, q: &str, v: Value) {
    let mut p = BTreeMap::new();
    p.insert("v".to_string(), v);
    match c.query_with_params(q, p) {
        Ok(r) => println!("  ok  {label}\n      -> {}", show(&r.rows)),
        Err(e) => println!("  ERR {label}\n      -> {e}"),
    }
}
fn fresh() -> (Database, Connection) {
    let db = Database::open(":memory:").unwrap();
    let c = db.connect().unwrap();
    (db, c)
}

fn main() {
    let (_db, mut c) = fresh();
    println!("== numbers");
    run(&mut c, "CREATE (:N {a: 9223372036854775807, b: -9223372036854775808})");
    run(&mut c, "MATCH (n:N) RETURN n.a, n.b");
    run(&mut c, "CREATE (:N2 {a: 9223372036854775808})");
    run(&mut c, "CREATE (:N3 {f: 1.0, g: 0.1, h: 1.0e300, i: -0.0})");
    run(&mut c, "MATCH (n:N3) RETURN n.f, n.g, n.h, n.i, toString(n.f), toString(n.h)");
    runp(&mut c, "NaN param", "CREATE (n:F {x: $v}) RETURN n.x", Value::Float(f64::NAN));
    runp(&mut c, "Inf param", "CREATE (n:F {x: $v}) RETURN n.x", Value::Float(f64::INFINITY));
    runp(&mut c, "-Inf param", "CREATE (n:F {x: $v}) RETURN n.x", Value::Float(f64::NEG_INFINITY));
    run(&mut c, "RETURN toString(1.5e-7), toString(123456789.123456789), toString(1e21)");
    println!("== strings");
    run(&mut c, "CREATE (:S {a: 'it\\'s', b: \"tab\\there\", c: 'é😀', d: ''})");
    run(&mut c, "MATCH (n:S) RETURN n.a, n.b, n.c, size(n.d)");
    runp(&mut c, "string with NUL and control chars", "CREATE (n:S {x: $v}) RETURN size(n.x)", Value::String("a\u{0}b\u{1}c\u{7f}".into()));
    println!("== lists");
    run(&mut c, "CREATE (:L {a: [1,2,3], b: ['x','y'], c: [true,false], d: [1.5, 2.5], e: []})");
    run(&mut c, "MATCH (n:L) RETURN n.a, n.b, n.c, n.d, n.e, valueType(n.e)");
    run(&mut c, "CREATE (:L2 {a: [1, 'x']})");
    run(&mut c, "CREATE (:L2 {a: [1, 2.5]})");
    run(&mut c, "CREATE (:L2 {a: [1, null]})");
    run(&mut c, "CREATE (:L2 {a: [[1],[2]]})");
    run(&mut c, "CREATE (:L2 {a: {k: 1}})");
    run(&mut c, "CREATE (:L2 {a: null})");
    run(&mut c, "MATCH (n:L2) RETURN n.a, valueType(n.a)");
    println!("== other storable types");
    run(&mut c, "CREATE (:T {d: date('2024-01-15'), dt: datetime('2024-01-15T10:30:00Z'), ldt: localdatetime('2024-01-15T10:30:00'), t: time('10:30:00Z'), lt: localtime('10:30:00'), du: duration('P1DT2H')})");
    run(&mut c, "MATCH (n:T) RETURN n.d, n.dt, n.ldt, n.t, n.lt, n.du");
    run(&mut c, "MATCH (n:T) RETURN toString(n.d), toString(n.dt), toString(n.ldt), toString(n.t), toString(n.lt), toString(n.du)");
    run(&mut c, "CREATE (:P {p: point({x: 1.0, y: 2.0}), q: point({latitude: 1.0, longitude: 2.0, crs: 'WGS-84'})})");
    run(&mut c, "MATCH (n:P) RETURN n.p, n.q");
    run(&mut c, "CREATE (:V {v: vector([1.0, 2.0, 3.0], 3, FLOAT32)})");
    run(&mut c, "CREATE (:U {u: randomUUID()})");
    run(&mut c, "MATCH (n:U) RETURN n.u, valueType(n.u)");
    runp(&mut c, "bytes param", "CREATE (n:B {x: $v}) RETURN n.x", Value::Bytes(Arc::from(vec![1u8,2,3])));
    run(&mut c, "CREATE (:T2 {l: [date('2024-01-15'), date('2024-02-01')]})");
    run(&mut c, "MATCH (n:T2) RETURN n.l, valueType(n.l)");
    println!("== keys");
    run(&mut c, "CREATE (:K {`with space`: 1, `a.b`: 2, `ünï`: 3, `x`: 4, `x`: 5})");
    run(&mut c, "MATCH (n:K) RETURN keys(n)");
    run(&mut c, "CREATE (:K2 {``: 1})");
    run(&mut c, "MATCH (n:K2) RETURN keys(n)");
    println!("== parameter-supplied values (what a loader would use)");
    runp(&mut c, "int", "CREATE (n:Q {x: $v}) RETURN valueType(n.x)", Value::Int(1));
    runp(&mut c, "list of ints", "CREATE (n:Q {x: $v}) RETURN valueType(n.x)", Value::List(Arc::new(vec![Value::Int(1), Value::Int(2)])));
    runp(&mut c, "empty list", "CREATE (n:Q {x: $v}) RETURN valueType(n.x)", Value::List(Arc::new(vec![])));
    runp(&mut c, "mixed list", "CREATE (n:Q {x: $v}) RETURN valueType(n.x)", Value::List(Arc::new(vec![Value::Int(1), Value::String("a".into())])));
    runp(&mut c, "int+float list", "CREATE (n:Q {x: $v}) RETURN n.x", Value::List(Arc::new(vec![Value::Int(1), Value::Float(2.5)])));
    runp(&mut c, "null in SET", "CREATE (n:Q) SET n.x = $v RETURN keys(n)", Value::Null);
}
