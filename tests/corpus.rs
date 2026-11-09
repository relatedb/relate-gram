//! Conformance with the tree-sitter-gram corpus (SC-001, ADR-007 section 2.6).
#[path = "support/corpus_reader.rs"]
mod corpus_reader;

#[path = "support/expectations.rs"]
mod expectations;
use corpus_reader::{read_corpus, CorpusCase};
use std::collections::{BTreeSet, HashMap};

fn fresh() -> (relate::Database, relate::Connection) {
    let db = relate::Database::open(":memory:").unwrap();
    let c = db.connect().unwrap();
    (db, c)
}

fn parse_tree(text: &str) -> tree_sitter::Tree {
    let mut p = tree_sitter::Parser::new();
    p.set_language(&tree_sitter_gram::LANGUAGE.into()).unwrap();
    p.parse(text, None).unwrap()
}

/// Node and relationship counts read straight off the tree-sitter tree, with
/// no rules applied: distinct identifiers plus anonymous elements.
fn reference_counts(text: &str) -> (usize, usize) {
    fn walk(
        n: tree_sitter::Node<'_>,
        src: &str,
        nodes: &mut (BTreeSet<String>, usize),
        rels: &mut (BTreeSet<String>, usize),
    ) {
        let take = |set: &mut (BTreeSet<String>, usize)| match n.child_by_field_name("identifier") {
            Some(i) => {
                set.0.insert(src[i.byte_range()].to_string());
            }
            None => set.1 += 1,
        };
        match n.kind() {
            "node_pattern" => take(nodes),
            "right_arrow" | "left_arrow" => take(rels),
            _ => {}
        }
        let mut c = n.walk();
        for ch in n.children(&mut c) {
            walk(ch, src, nodes, rels);
        }
    }
    let tree = parse_tree(text);
    let mut nodes = (BTreeSet::new(), 0);
    let mut rels = (BTreeSet::new(), 0);
    walk(tree.root_node(), text, &mut nodes, &mut rels);
    (nodes.0.len() + nodes.1, rels.0.len() + rels.1)
}

fn count(conn: &mut relate::Connection, q: &str) -> usize {
    match conn.query(q).unwrap().rows[0][0] {
        relate::Value::Int(i) => i as usize,
        _ => panic!("count"),
    }
}

/// Pair each corpus case with its expectation (by file, name and occurrence).
fn paired() -> Vec<(String, CorpusCase, expectations::Expectation)> {
    let table = expectations::read();
    let mut by_key: HashMap<(String, String), Vec<expectations::Expectation>> = HashMap::new();
    for e in table {
        by_key
            .entry((e.file.clone(), e.name.clone()))
            .or_default()
            .push(e);
    }
    let mut seen: HashMap<(String, String), usize> = HashMap::new();
    let mut out = Vec::new();
    for (file, cases) in read_corpus() {
        for case in cases {
            let key = (file.clone(), case.name.clone());
            let n = seen.entry(key.clone()).or_default();
            let exp = by_key
                .get(&key)
                .and_then(|v| v.get(*n))
                .unwrap_or_else(|| panic!("no expectation for {file}: {}", case.name))
                .clone();
            *n += 1;
            out.push((file.clone(), case, exp));
        }
    }
    out
}

#[test]
fn every_corpus_case_has_an_expectation_and_every_expectation_a_case() {
    let cases = paired().len();
    assert_eq!(cases, expectations::read().len());
}

#[test]
fn the_loader_agrees_with_the_expectations_on_every_corpus_case() {
    let mut problems = Vec::new();
    for (file, case, exp) in paired() {
        let (_db, mut conn) = fresh();
        let result = relate_gram::load(&mut conn, case.input.as_bytes(), &Default::default());
        let label = format!("{file}: {}", case.name);
        match (exp.expect.as_str(), result) {
            ("accept", Ok(_)) => {
                let (n, r) = reference_counts(&case.input);
                let got = (
                    count(&mut conn, "MATCH (n) RETURN count(n)"),
                    count(&mut conn, "MATCH ()-[r]->() RETURN count(r)"),
                );
                if got != (n, r) {
                    problems.push(format!(
                        "{label}: loaded {got:?}, the tree has {:?}",
                        (n, r)
                    ));
                }
            }
            ("accept", Err(f)) => {
                let codes: Vec<_> = f
                    .diagnostics
                    .iter()
                    .filter(|d| d.is_error())
                    .map(|d| d.code)
                    .collect();
                problems.push(format!("{label}: expected to load, failed with {codes:?}"));
            }
            ("reject", Ok(_)) => {
                problems.push(format!("{label}: expected {:?}, but it loaded", exp.codes))
            }
            ("reject", Err(f)) => {
                let codes: Vec<&str> = f
                    .diagnostics
                    .iter()
                    .filter(|d| d.is_error())
                    .map(|d| d.code)
                    .collect();
                if !codes.iter().any(|c| exp.codes.iter().any(|e| e == c)) {
                    problems.push(format!(
                        "{label}: expected one of {:?}, got {codes:?}",
                        exp.codes
                    ));
                }
                if f.diagnostics
                    .iter()
                    .any(|d| d.is_error() && d.span.is_some_and(|s| s.line == 0))
                {
                    problems.push(format!("{label}: an error has no position"));
                }
            }
            (other, _) => problems.push(format!("{label}: unknown expectation {other}")),
        }
    }
    assert!(
        problems.is_empty(),
        "{} mismatch(es):\n{}",
        problems.len(),
        problems.join("\n")
    );
}

/// The published `tree-sitter-gram` crate used by the build is the grammar the
/// corpus belongs to (ADR-007 section 2.6).
#[test]
fn the_crate_grammar_equals_the_submodule_grammar() {
    let path = format!(
        "{}/external/tree-sitter-gram/src/node-types.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let submodule = std::fs::read_to_string(path).unwrap();
    assert_eq!(tree_sitter_gram::NODE_TYPES.trim(), submodule.trim());
}

#[test]
fn the_crate_parses_every_corpus_input_as_the_corpus_says() {
    for (file, cases) in read_corpus() {
        for case in cases {
            let tree = parse_tree(&case.input);
            assert_eq!(
                tree.root_node().has_error(),
                case.is_error(),
                "{file}: {}",
                case.name
            );
        }
    }
}
