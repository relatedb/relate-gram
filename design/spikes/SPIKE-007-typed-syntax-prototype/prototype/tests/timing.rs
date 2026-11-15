//! A rough cost check: parse and lower 50,000 nodes with properties. Run with
//! `cargo test --release --test timing -- --ignored --nocapture`.
#[test]
#[ignore = "timing"]
fn parse_and_lower_fifty_thousand_nodes() {
    let mut text = String::new();
    for i in 0..50_000 {
        text.push_str(&format!("(n{i}:Person {{k: {i}, name: \"node {i}\", score: {i}.5}})\n"));
    }
    let t = std::time::Instant::now();
    let d = gramsyn::parse(&text);
    let parsed = t.elapsed();
    let t = std::time::Instant::now();
    let (patterns, diags) = gramsyn::project(&d);
    println!(
        "{} bytes: parse and lower {:?}, project {:?} ({} patterns, {} diagnostics)",
        text.len(),
        parsed,
        t.elapsed(),
        patterns.len(),
        diags.len()
    );
    assert!(d.diagnostics.is_empty());
}
