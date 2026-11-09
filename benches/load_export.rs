//! Load and export timings (SC-006). Run with `cargo bench --bench load_export`.
//! The same limit is asserted by `tests/perf.rs`.

use relate::Database;
use std::time::Instant;

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

fn main() {
    for (nodes, rels) in [(500, 500), (5_000, 5_000), (25_000, 25_000)] {
        for alternate in [false, true] {
            let db = Database::open(":memory:").unwrap();
            let mut conn = db.connect().unwrap();
            let text = gram(nodes, rels, alternate);
            let t = Instant::now();
            relate_gram::load(&mut conn, text.as_bytes(), &Default::default()).unwrap();
            let load = t.elapsed();
            let t = Instant::now();
            relate_gram::export(&mut conn, &Default::default()).unwrap();
            println!(
                "{:>6} elements, alternating {:<5}: load {:>10.1?}, export {:>10.1?}",
                nodes + rels,
                alternate,
                load,
                t.elapsed()
            );
        }
    }
}
