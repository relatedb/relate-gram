//! SPIKE-001 probe. Disposable. Prints what Relate does; asserts nothing.
use relate::{Connection, Database, Value};
use std::collections::BTreeMap;

fn show(r: &[Vec<Value>]) -> String {
    r.iter().map(|row| format!("{:?}", row)).collect::<Vec<_>>().join(" | ")
}

fn run(c: &mut Connection, q: &str) {
    match c.query(q) {
        Ok(r) => println!("  ok  {q}\n      -> {}", show(&r.rows)),
        Err(e) => println!("  ERR {q}\n      -> {e}"),
    }
}

fn rows(c: &mut Connection, q: &str) -> Vec<Vec<Value>> {
    c.query(q).map(|r| r.rows).unwrap_or_default()
}

fn plan(c: &mut Connection, q: &str) {
    match c.query(&format!("EXPLAIN {q}")) {
        Ok(r) => {
            let s = format!("{:?}", r.plan_root());
            let hits: Vec<&str> = ["IndexSeek", "UniqueIndexSeek", "NodeByLabelScan", "AllNodesScan", "Filter"]
                .into_iter().filter(|k| s.contains(k)).collect();
            println!("  plan {q}\n      -> operators seen: {hits:?}");
        }
        Err(e) => println!("  plan ERR {q}: {e}"),
    }
}

fn fresh() -> (Database, Connection) {
    let db = Database::open(":memory:").unwrap();
    let c = db.connect().unwrap();
    (db, c)
}

fn p1_at_id() {
    println!("\n== 1. `@id` as a property key");
    let (_db, mut c) = fresh();
    run(&mut c, "CREATE (:P {`@id`: 'a', name: 'Ada'})");
    run(&mut c, "MATCH (n:P) WHERE n.`@id` = 'a' RETURN n.`@id`, n.name");
    run(&mut c, "MATCH (n:P {`@id`: 'a'}) RETURN count(n)");
    run(&mut c, "MATCH (n:P) RETURN properties(n)");
    run(&mut c, "MATCH (n:P) RETURN keys(n)");
    run(&mut c, "MATCH (n:P) RETURN n");
    // from a parameter map
    let mut params = BTreeMap::new();
    params.insert("props".to_string(), Value::Map(std::sync::Arc::new(
        [("@id".to_string(), Value::String("b".into()))].into_iter().collect())));
    match c.query_with_params("CREATE (n:P) SET n = $props RETURN n.`@id`", params) {
        Ok(r) => println!("  ok  SET n = $props with @id\n      -> {}", show(&r.rows)),
        Err(e) => println!("  ERR SET n = $props with @id -> {e}"),
    }
    run(&mut c, "CREATE (a:Q {`@id`:'x'})-[:R {`@id`:'r1'}]->(b:Q {`@id`:'y'}) RETURN a.`@id`, b.`@id`");
    run(&mut c, "MATCH ()-[r:R]->() RETURN r.`@id`");
    run(&mut c, "CREATE INDEX p_atid FOR (n:P) ON (n.`@id`)");
    run(&mut c, "CREATE CONSTRAINT p_atid_u FOR (n:P) REQUIRE n.`@id` IS UNIQUE");
    run(&mut c, "CREATE (:P {`@id`: 'a'})"); // duplicate: should fail if constraint holds
    run(&mut c, "CREATE CONSTRAINT r_atid_u FOR ()-[r:R]-() REQUIRE r.`@id` IS UNIQUE");
    run(&mut c, "SHOW CONSTRAINTS");
    run(&mut c, "SHOW INDEXES");
    // a key that starts with @ and has other characters
    run(&mut c, "CREATE (:Z {`@hash`: 'sha256:x', `@@`: 1})");
    run(&mut c, "MATCH (n:Z) RETURN keys(n)");
}

fn p2_id_order() {
    println!("\n== 2. id() determinism and order");
    let seq = |c: &mut Connection| {
        for i in 0..5 { run_quiet(c, &format!("CREATE (:N {{seq:{i}}})")); }
        run_quiet(c, "UNWIND range(5,9) AS i CREATE (:N {seq:i})");
        run_quiet(c, "MATCH (a:N {seq:0}),(b:N {seq:1}) CREATE (a)-[:E {seq:100}]->(b)");
        run_quiet(c, "MATCH (a:N {seq:1}),(b:N {seq:2}) CREATE (a)-[:E {seq:101}]->(b)");
        rows(c, "MATCH (n:N) RETURN id(n), n.seq ORDER BY id(n)")
    };
    let (_d1, mut c1) = fresh();
    let (_d2, mut c2) = fresh();
    let r1 = seq(&mut c1);
    let r2 = seq(&mut c2);
    println!("  db1: {}", show(&r1));
    println!("  db2 equal to db1: {}", r1 == r2);
    let seqs: Vec<String> = r1.iter().map(|r| format!("{:?}", r[1])).collect();
    println!("  creation order == id order: {}", seqs == (0..10).map(|i| format!("{:?}", Value::Int(i))).collect::<Vec<_>>());
    println!("  rels: {}", show(&rows(&mut c1, "MATCH ()-[r:E]->() RETURN id(r), r.seq ORDER BY id(r)")));
    // deletion and reuse
    run(&mut c1, "MATCH (n:N) WHERE n.seq IN [2,3] DETACH DELETE n");
    run(&mut c1, "CREATE (:N {seq:10}), (:N {seq:11})");
    println!("  after delete+create: {}", show(&rows(&mut c1, "MATCH (n:N) RETURN id(n), n.seq ORDER BY id(n)")));
    // order of unordered scan vs ORDER BY id
    println!("  unordered scan : {}", show(&rows(&mut c1, "MATCH (n:N) RETURN n.seq")));
    // multi-label / mixed creates in one statement
    let (_d3, mut c3) = fresh();
    run(&mut c3, "CREATE (a:A)-[:R]->(b:B), (c:C)");
    run(&mut c3, "MATCH (n) RETURN id(n), labels(n) ORDER BY id(n)");
}

fn run_quiet(c: &mut Connection, q: &str) { let _ = c.query(q); }

fn p3_int_string() {
    println!("\n== 3. integer vs string identity");
    let (_db, mut c) = fresh();
    run(&mut c, "CREATE (:P {`@id`: '42'})");
    run(&mut c, "MATCH (n:P) WHERE n.`@id` = 42 RETURN count(n)");
    run(&mut c, "MATCH (n:P) WHERE n.`@id` = '42' RETURN count(n)");
    run(&mut c, "MATCH (n:P) WHERE n.`@id` = toString(42) RETURN count(n)");
    run(&mut c, "MERGE (n:P {`@id`: '42'}) RETURN count(n)");
    run(&mut c, "MERGE (n:P {`@id`: 42}) RETURN count(n)"); // different type: new node?
    run(&mut c, "MATCH (n:P) RETURN count(n)");
    // user identity property holding either type
    run(&mut c, "CREATE (:U {uid: 42}), (:U {uid: '42'}), (:U {uid: 7})");
    run(&mut c, "MATCH (n:U) WHERE toString(n.uid) = '42' RETURN count(n)");
    run(&mut c, "MATCH (n:U) WHERE n.uid = 42 RETURN count(n)");
    run(&mut c, "MATCH (n:U) RETURN n.uid, valueType(n.uid)");
    run(&mut c, "CREATE INDEX u_uid FOR (n:U) ON (n.uid)");
    plan(&mut c, "MATCH (n:U) WHERE n.uid = 42 RETURN n");
    plan(&mut c, "MATCH (n:U) WHERE toString(n.uid) = '42' RETURN n");
    // large and negative integers, and 1.0 vs 1
    run(&mut c, "RETURN toString(9007199254740993), toString(-0), toString(1.0)");
}

fn p4_atomic() {
    println!("\n== 4. transactions");
    let (_db, mut c) = fresh();
    println!("  in_transaction before begin: {}", c.in_transaction());
    run(&mut c, "CREATE CONSTRAINT k FOR (n:K) REQUIRE n.id IS UNIQUE");
    c.begin().map(|_| println!("  begin ok")).unwrap_or_else(|e| println!("  begin ERR {e}"));
    run(&mut c, "CREATE (:K {id: 1})");
    run(&mut c, "CREATE (:K {id: 2})");
    run(&mut c, "CREATE (:K {id: 1})"); // violates unique
    println!("  in_transaction after failed stmt: {}", c.in_transaction());
    run(&mut c, "MATCH (n:K) RETURN count(n)"); // does the txn still work, what does it see?
    match c.rollback() { Ok(_) => println!("  rollback ok"), Err(e) => println!("  rollback ERR {e}") }
    run(&mut c, "MATCH (n:K) RETURN count(n)"); // expect 0
    // commit path and visibility to another connection
    let (db, mut a) = fresh();
    let mut b = db.connect().unwrap();
    a.begin().unwrap();
    run(&mut a, "CREATE (:T {x:1})");
    println!("  other connection before commit: {}", show(&rows(&mut b, "MATCH (n:T) RETURN count(n)")));
    match a.commit() { Ok(bm) => println!("  commit ok bookmark={bm:?}"), Err(e) => println!("  commit ERR {e}") }
    println!("  other connection after commit : {}", show(&rows(&mut b, "MATCH (n:T) RETURN count(n)")));
    // error without a constraint: a runtime type error part-way
    let (_d, mut c2) = fresh();
    c2.begin().unwrap();
    run(&mut c2, "CREATE (:T {x:1})");
    run(&mut c2, "CREATE (:T {x: [1, [2]]})"); // nested list property: invalid
    println!("  in_transaction after type error: {}", c2.in_transaction());
    let _ = c2.rollback();
    run(&mut c2, "MATCH (n:T) RETURN count(n)");
    // drop without commit = implicit rollback
    let (db3, mut c3) = fresh();
    { let mut t = db3.connect().unwrap(); t.begin().unwrap(); run(&mut t, "CREATE (:D)"); }
    run(&mut c3, "MATCH (n:D) RETURN count(n)");
    // DDL inside a transaction
    let (_d4, mut c4) = fresh();
    c4.begin().unwrap();
    run(&mut c4, "CREATE CONSTRAINT q FOR (n:Q) REQUIRE n.id IS UNIQUE");
    let _ = c4.rollback();
    run(&mut c4, "SHOW CONSTRAINTS");
}

fn p5_constraints() {
    println!("\n== 5. identity constraints and indexes");
    let (_db, mut c) = fresh();
    // single-property UNIQUE ignores missing
    run(&mut c, "CREATE CONSTRAINT u1 FOR (n:A) REQUIRE n.id IS UNIQUE");
    run(&mut c, "CREATE (:A), (:A), (:A {id: 1})");
    run(&mut c, "CREATE (:A {id: 1})");
    // composite UNIQUE with a missing component
    run(&mut c, "CREATE CONSTRAINT u2 FOR (n:B) REQUIRE (n.k1, n.k2) IS UNIQUE");
    run(&mut c, "CREATE (:B {k1: 1}), (:B {k1: 1})");
    run(&mut c, "CREATE (:B {k1: 1, k2: 2}), (:B {k1: 1, k2: 2})");
    // NODE KEY
    run(&mut c, "CREATE CONSTRAINT nk FOR (n:C) REQUIRE (n.k1, n.k2) IS NODE KEY");
    run(&mut c, "CREATE (:C {k1: 1})");
    run(&mut c, "CREATE (:C {k1: 1, k2: 2})");
    run(&mut c, "CREATE (:C {k1: 1, k2: 2})");
    // key constraint on type: does UNIQUE distinguish 1 and '1'?
    run(&mut c, "CREATE (:A {id: '1'})");
    // relationship constraints
    run(&mut c, "CREATE CONSTRAINT ru FOR ()-[r:R]-() REQUIRE r.id IS UNIQUE");
    run(&mut c, "CREATE (:X)-[:R {id: 1}]->(:X), (:X)-[:R {id: 1}]->(:X)");
    run(&mut c, "CREATE CONSTRAINT rk FOR ()-[r:S]-() REQUIRE r.id IS NOT NULL");
    run(&mut c, "CREATE (:X)-[:S]->(:X)");
    // adding a constraint to existing violating data
    run(&mut c, "CREATE (:D {id: 1}), (:D {id: 1})");
    run(&mut c, "CREATE CONSTRAINT du FOR (n:D) REQUIRE n.id IS UNIQUE");
    // plans: does the constraint / index back a lookup?
    plan(&mut c, "MATCH (n:A {id: 1}) RETURN n");
    plan(&mut c, "MATCH (n:B {k1: 1, k2: 2}) RETURN n");
    plan(&mut c, "MERGE (n:A {id: 5}) RETURN n");
    plan(&mut c, "MATCH ()-[r:R {id: 1}]->() RETURN r");
    run(&mut c, "SHOW CONSTRAINTS");
    run(&mut c, "SHOW INDEXES");
    // MERGE on a unique key: idempotent?
    run(&mut c, "MERGE (n:A {id: 9}) RETURN id(n)");
    run(&mut c, "MERGE (n:A {id: 9}) RETURN id(n)");
    // one label vs several labels with the same key (phase 1 labels are per node)
    run(&mut c, "CREATE (:A:M {id: 20}), (:M {id: 20})");
}

fn plan_raw(c: &mut Connection, q: &str) {
    match c.query(&format!("EXPLAIN {q}")) {
        Ok(r) => {
            let s = format!("{:?}", r.plan_root());
            let ops: Vec<String> = s.split(|ch: char| !ch.is_alphanumeric()).filter(|w| {
                w.ends_with("Seek") || w.ends_with("Scan") || w.contains("Merge") || w.contains("Filter") || w.contains("Expand") || w.contains("Apply")
            }).map(String::from).collect();
            println!("  plan {q}\n      -> {ops:?}");
        }
        Err(e) => println!("  plan ERR {q}: {e}"),
    }
}

fn p6_plans() {
    println!("\n== 6. what backs MERGE, relationships, and typed-either lookups");
    let (_db, mut c) = fresh();
    run(&mut c, "CREATE CONSTRAINT a_id FOR (n:A) REQUIRE n.`@id` IS UNIQUE");
    run(&mut c, "CREATE CONSTRAINT r_id FOR ()-[r:R]-() REQUIRE r.`@id` IS UNIQUE");
    run(&mut c, "CREATE CONSTRAINT u_uid FOR (n:U) REQUIRE n.uid IS UNIQUE");
    plan_raw(&mut c, "MATCH (n:A {`@id`: 'x'}) RETURN n");
    plan_raw(&mut c, "MERGE (n:A {`@id`: 'x'}) RETURN n");
    plan_raw(&mut c, "MATCH (n:A) WHERE n.`@id` IN ['x','y'] RETURN n");
    plan_raw(&mut c, "MATCH (n:U) WHERE n.uid IN [42, '42'] RETURN n");
    plan_raw(&mut c, "MATCH (n:U) WHERE n.uid = 42 OR n.uid = '42' RETURN n");
    plan_raw(&mut c, "MATCH ()-[r:R {`@id`: 'x'}]->() RETURN r");
    plan_raw(&mut c, "MATCH (n) WHERE n.`@id` = 'x' RETURN n");
    plan_raw(&mut c, "UNWIND [{k:'x'},{k:'y'}] AS row MATCH (n:A {`@id`: row.k}) RETURN n");
    plan_raw(&mut c, "UNWIND [{k:'x'},{k:'y'}] AS row MERGE (n:A {`@id`: row.k}) RETURN n");
    run(&mut c, "CREATE (:U {uid: 42}), (:U {uid: '42'})");
    run(&mut c, "MATCH (n:U) WHERE n.uid IN [42, '42'] RETURN count(n)");
    // merge by @id twice, with a relationship between merged endpoints
    run(&mut c, "MERGE (a:A {`@id`:'a'}) MERGE (b:A {`@id`:'b'}) MERGE (a)-[r:R {`@id`:'r0'}]->(b) RETURN id(a), id(b), id(r)");
    run(&mut c, "MERGE (a:A {`@id`:'a'}) MERGE (b:A {`@id`:'b'}) MERGE (a)-[r:R {`@id`:'r0'}]->(b) RETURN id(a), id(b), id(r)");
    run(&mut c, "MATCH (n) RETURN count(n)");
    // labels unioned and property merge: SET n += map keeps unstated properties
    run(&mut c, "MERGE (n:A {`@id`:'a'}) SET n:Extra, n += {name:'Ada'} RETURN labels(n), properties(n)");
    run(&mut c, "MERGE (n:A {`@id`:'a'}) SET n += {age: 3} RETURN properties(n)");
    // an unlabelled node cannot be found by a label-scoped MERGE
    run(&mut c, "CREATE ({`@id`:'z'})");
    run(&mut c, "MERGE (n:A {`@id`:'z'}) RETURN id(n)");
}

fn p7_scale() {
    use std::time::Instant;
    println!("\n== 7. match-by-@id strategies at 50k nodes");
    let (_db, mut c) = fresh();
    let t = Instant::now();
    run_quiet(&mut c, "UNWIND range(1,50000) AS i CREATE (:A {`@id`: 'n' + toString(i)})");
    println!("  create 50k: {:?}", t.elapsed());
    plan_raw(&mut c, "MATCH (n) WHERE id(n) = 5 RETURN n");
    plan_raw(&mut c, "UNWIND [1,2,3] AS i MATCH (n) WHERE id(n) = i RETURN n");
    plan_raw(&mut c, "MATCH (n) WHERE id(n) IN [1,2,3] RETURN n");
    let t = Instant::now();
    let r = rows(&mut c, "MATCH (n) WHERE n.`@id` IS NOT NULL RETURN id(n), n.`@id`");
    println!("  one scan returning id+@id for {} nodes: {:?}", r.len(), t.elapsed());
    let t = Instant::now();
    run_quiet(&mut c, "UNWIND range(1,2000) AS i MATCH (n) WHERE n.`@id` = 'n' + toString(i) SET n.touched = true");
    println!("  2000 unlabelled lookups by @id (no index): {:?}", t.elapsed());
    let t = Instant::now();
    run_quiet(&mut c, "UNWIND range(1,50000) AS i MATCH (n) WHERE id(n) = i - 1 SET n.byid = true");
    println!("  50k updates by id(): {:?}", t.elapsed());
    run_quiet(&mut c, "CREATE CONSTRAINT a_id FOR (n:A) REQUIRE n.`@id` IS UNIQUE");
    let t = Instant::now();
    run_quiet(&mut c, "UNWIND range(1,50000) AS i MERGE (n:A {`@id`: 'n' + toString(i)}) SET n.merged = true");
    println!("  50k label+@id MERGEs with unique constraint: {:?}", t.elapsed());
    run(&mut c, "MATCH (n:A) RETURN count(n), count(n.merged), count(n.byid)");
}

fn main() {
    p7_scale();
    p6_plans();
    p1_at_id();
    p2_id_order();
    p3_int_string();
    p4_atomic();
    p5_constraints();
}
