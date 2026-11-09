use gram_codec::*;
fn ts(label: &str, src: &str) {
    let mut p = tree_sitter::Parser::new();
    p.set_language(&tree_sitter_gram::LANGUAGE.into()).unwrap();
    let t = p.parse(src, None).unwrap();
    let r = t.root_node();
    println!("== {label}: {src:?}\n  ts has_error={}", r.has_error());
    println!("  {}", r.to_sexp().replace('\n'," "));
    fn walk(n: tree_sitter::Node, src: &str) { if n.kind()=="string_content" { println!("  string_content {:?} at {}:{}", &src[n.byte_range()], n.start_position().row+1, n.start_position().column+1); } let mut c=n.walk(); for ch in n.children(&mut c) { walk(ch, src); } }
    walk(r, src);
    let lowered = lower(parse_gram_cst(src).tree);
    println!("  lowered: {:?}", lowered.iter().map(|p| p.value.properties.clone()).collect::<Vec<_>>());
}
fn main() {
    ts("empty array", "(a {xs: []})");
    ts("missing value", "(a {k: })");
    ts("octal", "(a {n: 010})");
    ts("octal 08", "(a {n: 08})");
    ts("dup key", "(a {k: 1, k: 2})");
    ts("fenced tag", "(a {s: ```md\nbody\n```})");
    ts("fenced backslash", "(a {s: ```\na\\nb\n```})");
    ts("raw newline dq", "(a {s: \"a\nb\"})");
    ts("raw newline sq", "(a {s: 'a\nb'})");
    ts("raw newline bt", "(a {s: `a\nb`})");
    ts("raw CR dq", "(a {s: \"a\rb\"})");
    ts("raw 0x01 dq", "(a {s: \"a\u{1}b\"})");
    ts("raw DEL dq", "(a {s: \"a\u{7f}b\"})");
    ts("empty key", "(a {\"\": 1})");
    ts("i64 overflow", "(a {n: 9223372036854775808})");
}
