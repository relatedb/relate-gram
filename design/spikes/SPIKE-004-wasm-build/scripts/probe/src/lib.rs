//! Spike probe: parse gram with tree-sitter-gram and report the shape of the tree.
use tree_sitter::Parser;
use wasm_bindgen::prelude::*;

/// "<node count> <error count>" for `source`.
#[wasm_bindgen]
pub fn probe(source: &str) -> String {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_gram::LANGUAGE.into())
        .expect("load the gram grammar");
    let tree = parser.parse(source, None).expect("parse");
    let mut cursor = tree.walk();
    let (mut nodes, mut errors) = (0usize, 0usize);
    'walk: loop {
        let n = cursor.node();
        nodes += 1;
        if n.is_error() || n.is_missing() {
            errors += 1;
        }
        if cursor.goto_first_child() {
            continue;
        }
        loop {
            if cursor.goto_next_sibling() {
                break;
            }
            if !cursor.goto_parent() {
                break 'walk;
            }
        }
    }
    format!("{nodes} {errors}")
}
