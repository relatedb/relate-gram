//! Every diagnostic code is asserted by at least one test (T067, FR-015).

use relate_gram::diagnostics::code;
use std::path::Path;

/// Codes with no test, and why.
const UNTESTED: &[(&str, &str)] = &[(
    code::CONSTRUCT_UNSUPPORTED,
    "a guard against a grammar upgrade; no input of tree-sitter-gram 0.6.1 reaches it, and a \
     tree-sitter node cannot be built by hand",
)];

fn collect(dir: &Path, out: &mut String) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            if path.ends_with("tests/fixtures/valid") || path.ends_with("support") {
                continue;
            }
            collect(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs" || e == "expect")
            && !path.ends_with("coverage.rs")
        {
            out.push_str(&std::fs::read_to_string(&path).unwrap_or_default());
            out.push('\n');
        }
    }
}

#[test]
fn every_code_is_asserted_by_a_test() {
    let mut text = String::new();
    collect(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests"),
        &mut text,
    );
    let mut missing = Vec::new();
    for c in code::ALL {
        let constant = c.replace(['.', '-'], "_").to_uppercase();
        let mentioned = text.contains(&format!("\"{c}\""))
            || text.contains(&format!("code::{constant}"))
            || text.lines().any(|l| l.starts_with(&format!("{c} ")));
        let allowed = UNTESTED.iter().any(|(u, _)| u == c);
        if !mentioned && !allowed {
            missing.push(*c);
        }
    }
    assert!(missing.is_empty(), "codes with no test: {missing:?}");
}
