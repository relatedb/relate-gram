//! Reference run: the C runtime, on every file given. Prints "<file> <nodes> <errors>".
use tree_sitter::Parser;
fn main() {
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_gram::LANGUAGE.into()).unwrap();
    for path in std::env::args().skip(1) {
        let bytes = std::fs::read(&path).unwrap();
        let src = String::from_utf8_lossy(&bytes);
        let tree = parser.parse(src.as_ref(), None).unwrap();
        let mut c = tree.walk();
        let (mut n, mut e) = (0usize, 0usize);
        'w: loop {
            let node = c.node();
            n += 1;
            if node.is_error() || node.is_missing() { e += 1; }
            if c.goto_first_child() { continue; }
            loop {
                if c.goto_next_sibling() { break; }
                if !c.goto_parent() { break 'w; }
            }
        }
        println!("{} {} {}", std::path::Path::new(&path).file_name().unwrap().to_string_lossy(), n, e);
    }
}
