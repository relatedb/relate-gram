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
