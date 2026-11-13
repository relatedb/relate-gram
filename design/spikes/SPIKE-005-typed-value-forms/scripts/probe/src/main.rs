//! SPIKE-005 probe. Disposable. Prints what Relate 1.0.1 does with typed property values.
use relate::{Connection, Database, Value};

fn show(r: &[Vec<Value>]) -> String {
    r.iter().map(|row| format!("{:?}", row)).collect::<Vec<_>>().join(" | ")
}
fn run(c: &mut Connection, q: &str) {
    match c.query(q) {
        Ok(r) => println!("  ok  {q}\n      -> {}", show(&r.rows)),
        Err(e) => println!("  ERR {q}\n      -> {e}"),
    }
}

/// Store `ctor` as a property, print it, and rebuild it from its own text.
fn round(c: &mut Connection, label: &str, ctor: &str, reparse: &str) {
    println!("-- {label}: {ctor}");
    run(c, &format!("CREATE (:T {{v: {ctor}}})"));
    run(c, "MATCH (n:T) RETURN n.v, toString(n.v), valueType(n.v)");
    // reparse: a template where $S is replaced by the printed text, compared to the stored value
    run(c, &format!("MATCH (n:T) WITH n.v AS v RETURN v = {reparse} AS same_after_toString"));
    run(c, "MATCH (n:T) DELETE n");
}

fn main() {
    let db = Database::open(":memory:").unwrap();
    let mut c = db.connect().unwrap();

    println!("== date");
    round(&mut c, "date", "date('2024-01-15')", "date(toString(v))");
    round(&mut c, "date neg year", "date('-0044-03-15')", "date(toString(v))");
    round(&mut c, "date week form", "date('2024-W03-1')", "date(toString(v))");
    println!("== time / localtime");
    round(&mut c, "time Z", "time('10:30:00Z')", "time(toString(v))");
    round(&mut c, "time offset", "time('10:30:00+01:00')", "time(toString(v))");
    round(&mut c, "time nanos", "time('10:30:00.123456789+05:30')", "time(toString(v))");
    round(&mut c, "localtime", "localtime('10:30:00')", "localtime(toString(v))");
    round(&mut c, "localtime nanos", "localtime('10:30:00.123456789')", "localtime(toString(v))");
    println!("== datetime / localdatetime");
    round(&mut c, "datetime Z", "datetime('2024-01-15T10:30:00Z')", "datetime(toString(v))");
    round(&mut c, "datetime offset", "datetime('2024-01-15T10:30:00+01:00')", "datetime(toString(v))");
    round(&mut c, "datetime zone", "datetime('2024-01-15T10:30:00[Europe/Paris]')", "datetime(toString(v))");
    round(&mut c, "datetime offset+zone", "datetime('2024-01-15T10:30:00+01:00[Europe/Paris]')", "datetime(toString(v))");
    round(&mut c, "datetime nanos", "datetime('2024-01-15T10:30:00.123456789Z')", "datetime(toString(v))");
    round(&mut c, "localdatetime", "localdatetime('2024-01-15T10:30:00')", "localdatetime(toString(v))");
    round(&mut c, "localdatetime nanos", "localdatetime('2024-01-15T10:30:00.123456789')", "localdatetime(toString(v))");
    println!("== duration");
    round(&mut c, "duration", "duration('P1DT2H')", "duration(toString(v))");
    round(&mut c, "duration mixed", "duration('P1Y2M3DT4H5M6.789S')", "duration(toString(v))");
    round(&mut c, "duration negative", "duration('-P1DT2H')", "duration(toString(v))");
    round(&mut c, "duration weeks", "duration('P2W')", "duration(toString(v))");
    round(&mut c, "duration nanos", "duration('PT0.123456789S')", "duration(toString(v))");
    println!("== point");
    round(&mut c, "point 2D cartesian", "point({x: 1.0, y: 2.0})", "point({x: v.x, y: v.y, srid: v.srid})");
    round(&mut c, "point 3D cartesian", "point({x: 1.0, y: 2.0, z: 3.0})", "point({x: v.x, y: v.y, z: v.z, srid: v.srid})");
    round(&mut c, "point WGS-84 2D", "point({latitude: 40.7, longitude: -74.0})", "point({longitude: v.longitude, latitude: v.latitude, srid: v.srid})");
    round(&mut c, "point WGS-84 3D", "point({latitude: 40.7, longitude: -74.0, height: 10.0})", "point({longitude: v.longitude, latitude: v.latitude, height: v.height, srid: v.srid})");
    round(&mut c, "point srid", "point({x: 1.0, y: 2.0, srid: 7203})", "point({x: v.x, y: v.y, srid: v.srid})");
    run(&mut c, "RETURN point({x: 1.0, y: 2.0}) AS p, toString(point({x: 1.0, y: 2.0})) AS s");
    run(&mut c, "RETURN point({x: 1.0, y: 2.0}).srid, point({latitude: 1.0, longitude: 2.0}).srid");
    println!("== vector");
    run(&mut c, "RETURN vector([1, 2, 3], 3, FLOAT32) AS v, toString(vector([1, 2, 3], 3, FLOAT32)) AS s");
    run(&mut c, "RETURN vector('[1,2,3]', 3, FLOAT32) AS v");
    run(&mut c, "RETURN vector([1, 2, 3], 3, INTEGER8) AS v, valueType(vector([1,2,3], 3, INTEGER8))");
    run(&mut c, "CREATE (:V {v: vector([1.5, 2.5], 2, FLOAT32)})");
    run(&mut c, "MATCH (n:V) RETURN n.v, toString(n.v), valueType(n.v), vector_dimension_count(n.v)");
    println!("== uuid");
    run(&mut c, "RETURN randomUUID() AS u, valueType(randomUUID())");
    run(&mut c, "CREATE (:U {u: uuid('123e4567-e89b-12d3-a456-426614174000')})");
    run(&mut c, "MATCH (n:U) RETURN n.u, toString(n.u), valueType(n.u)");
    run(&mut c, "RETURN uuid('123E4567-E89B-12D3-A456-426614174000') = uuid('123e4567-e89b-12d3-a456-426614174000')");
    println!("== malformed text");
    for q in [
        "RETURN date('not a date')", "RETURN date('2024-13-45')", "RETURN time('25:00')",
        "RETURN datetime('2024-01-15')", "RETURN duration('1 day')", "RETURN uuid('xyz')",
        "RETURN point({x: 1.0})", "RETURN point('POINT(1 2)')", "RETURN date('')",
    ] { run(&mut c, q); }
    println!("== lists of typed values");
    run(&mut c, "CREATE (:LT {a: [date('2024-01-15'), date('2024-02-01')]})");
    run(&mut c, "MATCH (n:LT) RETURN n.a, valueType(n.a)");
    run(&mut c, "CREATE (:LT2 {a: [date('2024-01-15'), datetime('2024-01-15T10:30:00Z')]})");
    run(&mut c, "CREATE (:LT3 {a: [point({x: 1.0, y: 2.0})]})");
    run(&mut c, "CREATE (:LT4 {a: [duration('P1D'), duration('P2D')]})");
    run(&mut c, "MATCH (n:LT4) RETURN n.a, valueType(n.a)");
    println!("== follow-up probes");
    run(&mut c, "MATCH (n:LT2) RETURN n.a, valueType(n.a)");
    run(&mut c, "MATCH (n:LT3) RETURN n.a, valueType(n.a)");
    run(&mut c, "RETURN point({x: 1.0, y: 2.0, srid: 1234}) AS p");
    run(&mut c, "RETURN point({x: 1.0, y: 2.0, crs: 'wgs-84'}) AS p");
    run(&mut c, "RETURN point({longitude: 200.0, latitude: 95.0}) AS p");
    run(&mut c, "RETURN datetime('2024-01-15T10:30:00') AS p");
    run(&mut c, "RETURN time('10:30:00') AS p");
    run(&mut c, "RETURN date('2024-01-15T10:30:00') AS p");
    let mut params = std::collections::BTreeMap::new();
    params.insert("s".to_string(), Value::String("2024-01-15".into()));
    match c.query_with_params("CREATE (n:PP {d: date($s)}) RETURN n.d", params) {
        Ok(r) => println!("  ok  date($s) in CREATE\n      -> {}", show(&r.rows)),
        Err(e) => println!("  ERR date($s) in CREATE\n      -> {e}"),
    }
    println!("== empty list");
    run(&mut c, "CREATE (:E {a: []})");
    run(&mut c, "MATCH (n:E) RETURN n.a, valueType(n.a), size(n.a)");
    println!("== runtime-only values as properties");
    run(&mut c, "CREATE (:M {a: {k: 1}})");
    run(&mut c, "CREATE (:M {a: null})");
    run(&mut c, "MATCH (n:M) RETURN n");
}
