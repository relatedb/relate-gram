//! ADR-014 section 2.2: an `embedding` tagged string loads as a Relate vector.
use super::common::*;
use gram_value::tags::{interpret_tag, Typed};
use relate::Value;

/// The `embedding` contents in a fixture, in order.
fn contents(name: &str) -> Vec<String> {
    let text = String::from_utf8(fixture("valid", name)).unwrap();
    text.lines()
        .filter(|l| !l.starts_with("//"))
        .flat_map(|l| {
            l.split("embedding`")
                .skip(1)
                .map(|rest| rest.split('`').next().unwrap().to_string())
                .collect::<Vec<_>>()
        })
        .collect()
}

/// The elements of an embedding read straight from its bytes, independently of the crate's code.
fn expected_elements(content: &str) -> Vec<f64> {
    let Ok(Some(i)) = interpret_tag("embedding", content) else {
        panic!("embedding content");
    };
    let Typed::Embedding(e) = i.typed else {
        panic!("embedding")
    };
    match e.dtype().as_str() {
        "F32" => e
            .bytes()
            .chunks(4)
            .map(|c| f64::from(f32::from_le_bytes(c.try_into().unwrap())))
            .collect(),
        "F64" => e
            .bytes()
            .chunks(8)
            .map(|c| f64::from_le_bytes(c.try_into().unwrap()))
            .collect(),
        other => panic!("no reader for {other} here"),
    }
}

fn vector(v: &Value) -> (String, Vec<f64>) {
    let Value::Vector { coord, values } = v else {
        panic!("expected a vector, got {v:?}");
    };
    let floats = values
        .iter()
        .map(|x| match x {
            Value::Float(f) => *f,
            other => panic!("expected a float element, got {other:?}"),
        })
        .collect();
    (format!("{coord:?}"), floats)
}

#[test]
fn an_f32_embedding_loads_as_a_float32_vector_on_a_node_and_a_relationship() {
    let (_db, mut c) = fresh_db();
    load_fixture(&mut c, "valid", "embedding-f32").unwrap();
    let want = expected_elements(&contents("embedding-f32")[0]);
    assert_eq!(want, vec![1.0, -2.0]);
    let (kind, got) = vector(&node_prop(&mut c, "a", "e"));
    assert_eq!((kind.as_str(), &got), ("Float32", &want));
    let rows = run(&mut c, "MATCH ()-[r:R]->() RETURN r.e");
    let (kind, got) = vector(&rows[0][0]);
    assert_eq!((kind.as_str(), &got), ("Float32", &want));
}

#[test]
fn an_f64_embedding_loads_as_a_float64_vector() {
    let (_db, mut c) = fresh_db();
    load_fixture(&mut c, "valid", "embedding-f64").unwrap();
    let want = expected_elements(&contents("embedding-f64")[0]);
    assert_eq!(want.len(), 3);
    let (kind, got) = vector(&node_prop(&mut c, "a", "e"));
    assert_eq!((kind.as_str(), &got), ("Float64", &want));
}

#[test]
fn content_in_another_spelling_gives_the_same_vector() {
    let (_db, mut c) = fresh_db();
    load_fixture(&mut c, "valid", "embedding-nonnormal").unwrap();
    let (kind, got) = vector(&node_prop(&mut c, "a", "e"));
    assert_eq!((kind.as_str(), got), ("Float32", vec![1.0, -2.0]));
}

#[test]
fn the_smallest_dimension_loads() {
    let (_db, mut c) = fresh_db();
    load_fixture(&mut c, "valid", "embedding-dim-1").unwrap();
    let (kind, got) = vector(&node_prop(&mut c, "a", "e"));
    assert_eq!(kind, "Float32");
    assert_eq!(got.len(), 1);
}

#[test]
fn embedding_is_not_reported_as_a_dropped_tag_but_other_tags_still_are() {
    let (_db, mut c) = fresh_db();
    let e = &contents("embedding-f32")[0];
    let r = load_str(&mut c, &format!("(a {{e: embedding`{e}`, m: md`x`}})")).unwrap();
    let dropped: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == "value.tag-dropped")
        .collect();
    assert_eq!(dropped.len(), 1, "{:?}", r.diagnostics);
    assert!(dropped[0].message.contains("md"), "{}", dropped[0].message);
    assert!(matches!(node_prop(&mut c, "a", "e"), Value::Vector { .. }));
}

#[test]
fn loading_twice_keeps_one_vector() {
    // Loading the same file twice matches by identifier and keeps one vector.
    let (_db, mut c) = fresh_db();
    load_fixture(&mut c, "valid", "embedding-f32").unwrap();
    load_fixture(&mut c, "valid", "embedding-f32").unwrap();
    let rows = run(&mut c, "MATCH (n {`@id`: 'a'}) RETURN n.e");
    assert_eq!(rows.len(), 1);
}

// --- rejections (ADR-014 section 2.2) ---------------------------------------------

use relate_gram::settings::LoadSettings;

fn invalid(name: &str) -> Vec<u8> {
    fixture("invalid", name)
}

#[test]
fn check_and_load_give_the_same_error_for_each_rejected_embedding() {
    for name in [
        "embedding-dtype-i32",
        "embedding-dtype-case",
        "embedding-bad-b64",
        "embedding-empty",
        "embedding-dim-4097",
        "embedding-nan",
        "embedding-array",
    ] {
        let src = invalid(name);
        let checked = relate_gram::check(&src, &LoadSettings::default());
        let errs: Vec<_> = checked
            .diagnostics
            .iter()
            .filter(|d| d.is_error())
            .collect();
        assert_eq!(errs.len(), 1, "{name}: {:?}", checked.diagnostics);
        assert_eq!(errs[0].code, "value.tag-content", "{name}");
        let (_db, mut c) = fresh_db();
        let f = relate_gram::load(&mut c, &src, &LoadSettings::default()).unwrap_err();
        assert_eq!(codes(&f.diagnostics), vec!["value.tag-content"], "{name}");
        assert_eq!(f.diagnostics[0].span, errs[0].span, "{name}");
        let rows = run(&mut c, "MATCH (n) RETURN count(n)");
        assert_eq!(rows[0][0], Value::Int(0), "{name}: nothing is stored");
    }
}

#[test]
fn the_limit_messages_say_whose_limit_it_is() {
    let dim = relate_gram::check(&invalid("embedding-dim-4097"), &LoadSettings::default());
    let m = &dim.diagnostics[0].message;
    assert!(m.contains("4096") && m.contains("allows more"), "{m}");
    let nan = relate_gram::check(&invalid("embedding-nan"), &LoadSettings::default());
    let m = &nan.diagnostics[0].message;
    assert!(m.contains("index 1") && m.contains("Relate"), "{m}");
}

#[test]
fn the_dimension_limit_itself_loads() {
    use gram_value::tags::{Embedding, EmbeddingDtype};
    let bytes = vec![0u8; 4096 * 4];
    let content = Embedding::new(EmbeddingDtype::F32, 4096, bytes)
        .unwrap()
        .to_normal_content();
    let (_db, mut c) = fresh_db();
    load_str(&mut c, &format!("(a {{e: embedding`{content}`}})")).unwrap();
    let Value::Vector { values, .. } = node_prop(&mut c, "a", "e") else {
        panic!("vector");
    };
    assert_eq!(values.len(), 4096);
}

#[test]
fn an_array_of_embeddings_is_one_error_at_the_array_whatever_the_elements_hold() {
    let src = String::from_utf8(invalid("embedding-array")).unwrap();
    let checked = relate_gram::check(src.as_bytes(), &LoadSettings::default());
    assert_eq!(checked.diagnostics.len(), 1, "{:?}", checked.diagnostics);
    let d = &checked.diagnostics[0];
    assert!(
        d.message.contains("an array of embeddings"),
        "{}",
        d.message
    );
    // The error is at the array's `[`, not at an element.
    let at = d.span.unwrap().start;
    assert_eq!(&src[at..at + 1], "[");
    // Bad elements do not add errors of their own: still one.
    let bad = "(a {es: [embedding`QAAA!`, embedding`QAAA!`]})";
    let r = relate_gram::check(bad.as_bytes(), &LoadSettings::default());
    assert_eq!(r.diagnostics.len(), 1, "{:?}", r.diagnostics);
}

#[test]
fn an_embedding_mixed_with_another_value_is_an_ordinary_mixed_array() {
    let r = relate_gram::check(&invalid("embedding-array-mixed"), &LoadSettings::default());
    assert_eq!(codes(&r.diagnostics), vec!["value.array-mixed"]);
    assert!(r.diagnostics[0].message.contains("embedding"));
}
