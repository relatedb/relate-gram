//! SC-006: 10,000 elements load, and a database of that size exports, in under
//! ten seconds. Ignored by default; run with
//! `cargo test --release --test perf -- --ignored --nocapture`.

use relate::Database;
use std::time::{Duration, Instant};

const LIMIT: Duration = Duration::from_secs(10);

/// `nodes` nodes and `rels` relationships. With `alternate`, label sets and
/// relationship types change on every element, the worst case for batching
/// (research R5).
fn gram(nodes: usize, rels: usize, alternate: bool) -> String {
    let mut s = String::new();
    for i in 0..nodes {
        let label = if alternate {
            format!("L{}", i % 2)
        } else {
            "L".to_string()
        };
        s.push_str(&format!("(n{i}:{label} {{k: {i}, name: \"node {i}\"}})\n"));
    }
    for i in 0..rels {
        let ty = if alternate {
            format!("T{}", i % 2)
        } else {
            "T".to_string()
        };
        s.push_str(&format!(
            "(n{})-[r{i}:{ty} {{w: {i}}}]->(n{})\n",
            i % nodes,
            (i + 1) % nodes
        ));
    }
    s
}

fn measure(nodes: usize, rels: usize, alternate: bool) -> (Duration, Duration) {
    let db = Database::open(":memory:").unwrap();
    let mut conn = db.connect().unwrap();
    let text = gram(nodes, rels, alternate);
    let t = Instant::now();
    relate_gram::load(&mut conn, text.as_bytes(), &Default::default()).expect("load");
    let load = t.elapsed();
    let t = Instant::now();
    let out = relate_gram::export(&mut conn, &Default::default()).expect("export");
    let export = t.elapsed();
    assert!(out.lines().count() >= nodes + rels);
    println!(
        "{nodes} nodes + {rels} relationships (alternating: {alternate}): load {load:?}, export {export:?}"
    );
    (load, export)
}

#[test]
#[ignore = "performance check; run with --ignored"]
fn ten_thousand_elements_load_and_export_within_the_limit() {
    for alternate in [false, true] {
        let (load, export) = measure(5_000, 5_000, alternate);
        assert!(load < LIMIT, "load took {load:?}");
        assert!(export < LIMIT, "export took {export:?}");
    }
}

#[test]
#[ignore = "performance check; run with --ignored"]
fn fifty_thousand_elements_are_measured() {
    // SPIKE-001 measured scans to this size; no limit is asserted, the numbers are printed.
    measure(25_000, 25_000, false);
}

/// 10,000 elements, each carrying a date, a duration and a point (specs/002-typed-tags SC-004),
/// or the same properties as plain strings.
fn typed_gram(nodes: usize, typed: bool) -> String {
    let mut s = String::new();
    for i in 0..nodes {
        let (d, du, p) = if typed {
            (
                format!("date`2024-01-{:02}`", i % 28 + 1),
                format!("duration`PT{}S`", i % 1000 + 1),
                format!("point`SRID=7203;POINT({}.5 {}.5)`", i % 90, i % 45),
            )
        } else {
            (
                format!("\"2024-01-{:02}\"", i % 28 + 1),
                format!("\"PT{}S\"", i % 1000 + 1),
                format!("\"{}.5 {}.5\"", i % 90, i % 45),
            )
        };
        s.push_str(&format!("(n{i}:L {{d: {d}, du: {du}, p: {p}}})\n"));
    }
    s
}

#[test]
#[ignore = "performance check; run with --ignored"]
fn typed_values_cost_no_more_than_ten_times_plain_strings_to_load() {
    let mut times = Vec::new();
    for typed in [false, true] {
        let db = Database::open(":memory:").unwrap();
        let mut conn = db.connect().unwrap();
        let text = typed_gram(10_000, typed);
        let t = Instant::now();
        relate_gram::load(&mut conn, text.as_bytes(), &Default::default()).expect("load");
        let load = t.elapsed();
        let t = Instant::now();
        let out = relate_gram::export(&mut conn, &Default::default()).expect("export");
        let export = t.elapsed();
        assert!(out.lines().count() >= 10_000);
        println!("10,000 nodes, typed {typed}: load {load:?}, export {export:?}");
        times.push((load, export));
    }
    let ratio = times[1].0.as_secs_f64() / times[0].0.as_secs_f64();
    println!("typed load / plain load = {ratio:.2} (SC-004 aims for at most 3; this test fails above 10)");
    assert!(ratio < 10.0, "typed load took {ratio:.1} times as long");
    assert!(times[1].0 < LIMIT && times[1].1 < LIMIT);
}
