//! ADR-014 section 2.3: float vectors are written as `embedding`; integer vectors are refused.
use super::common::*;
use gram_value::tags::{Embedding, EmbeddingDtype};

fn expected_text(dtype: EmbeddingDtype, bytes: Vec<u8>, dim: usize) -> String {
    Embedding::new(dtype, dim, bytes)
        .unwrap()
        .to_value()
        .to_text()
        .unwrap()
}

#[test]
fn float32_and_float64_vectors_are_written_as_embedding() {
    let (_db, mut c) = fresh_db();
    run(
        &mut c,
        "CREATE (:P {f32: vector([1.0, -2.0], 2, FLOAT32), f64: vector([0.1, 2.5, -3.0], 3, FLOAT64)})",
    );
    let text = export_str(&mut c).unwrap();
    let f32_bytes = [1.0f32.to_le_bytes(), (-2.0f32).to_le_bytes()].concat();
    let f64_bytes = [0.1f64, 2.5, -3.0]
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect::<Vec<u8>>();
    let want32 = expected_text(EmbeddingDtype::F32, f32_bytes, 2);
    let want64 = expected_text(EmbeddingDtype::F64, f64_bytes, 3);
    assert!(text.contains(&format!("f32: {want32}")), "{text}");
    assert!(text.contains(&format!("f64: {want64}")), "{text}");
}

#[test]
fn reference_writer_content_is_written_back_byte_for_byte() {
    let (_db, mut c) = fresh_db();
    load_fixture(&mut c, "valid", "embedding-f32").unwrap();
    let source = String::from_utf8(fixture("valid", "embedding-f32")).unwrap();
    let content = source
        .split("embedding`")
        .nth(1)
        .unwrap()
        .split('`')
        .next()
        .unwrap()
        .to_string();
    let text = export_str(&mut c).unwrap();
    assert!(text.contains(&format!("embedding`{content}`")), "{text}");
}

#[test]
fn content_in_another_spelling_is_written_in_normal_form_with_the_same_bytes() {
    let (_db, mut c) = fresh_db();
    load_fixture(&mut c, "valid", "embedding-nonnormal").unwrap();
    let reference = String::from_utf8(fixture("valid", "embedding-f32")).unwrap();
    let normal = reference
        .split("embedding`")
        .nth(1)
        .unwrap()
        .split('`')
        .next()
        .unwrap();
    let text = export_str(&mut c).unwrap();
    assert!(text.contains(&format!("embedding`{normal}`")), "{text}");
}

#[test]
fn integer_vectors_are_refused_with_a_reason_naming_the_property() {
    let (_db, mut c) = fresh_db();
    for (i, kind) in ["INTEGER8", "INTEGER16", "INTEGER32", "INTEGER64"]
        .iter()
        .enumerate()
    {
        run(
            &mut c,
            &format!("CREATE (:P {{k{i}: vector([1, 2], 2, {kind})}})"),
        );
    }
    let f = export_str(&mut c).unwrap_err();
    assert_eq!(codes_of(&f), vec!["export.value"; 4]);
    for d in &f.diagnostics {
        assert!(d.message.contains("float vectors only"), "{}", d.message);
    }
    for i in 0..4 {
        assert!(
            f.diagnostics
                .iter()
                .any(|d| d.message.contains(&format!("k{i}"))),
            "k{i} not named"
        );
    }
}

#[test]
fn a_database_with_float_and_integer_vectors_lists_only_the_integer_ones() {
    let (_db, mut c) = fresh_db();
    run(
        &mut c,
        "CREATE (:P {ok: vector([1.0], 1, FLOAT32), bad: vector([1], 1, INTEGER8)})",
    );
    let f = export_str(&mut c).unwrap_err();
    assert_eq!(f.diagnostics.len(), 1);
    assert!(
        f.diagnostics[0].message.contains("bad"),
        "{}",
        f.diagnostics[0].message
    );
}

#[test]
fn other_values_are_written_as_before() {
    let (_db, mut c) = fresh_db();
    run(&mut c, "CREATE (:P {s: 'active', n: 1, f: 1.5, l: [1, 2]})");
    let text = export_str(&mut c).unwrap();
    assert!(
        text.contains("s: active") && text.contains("l: [1, 2]"),
        "{text}"
    );
}
