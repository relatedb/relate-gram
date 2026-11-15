//! ADR-014 sections 2.2 and 2.4: 16-bit embeddings, with Relate's preview flag on.
//!
//! The flag is a process environment variable, so these tests live in their own test target and
//! set it before the first database is opened; `embedding16_off.rs` is the same without it.
mod common;
use common::*;
use gram_value::tags::{Embedding, EmbeddingDtype};
use relate::Value;
use std::sync::Once;

static FLAG: Once = Once::new();

fn db() -> (relate::Database, relate::Connection) {
    FLAG.call_once(|| std::env::set_var("RELATE_PREVIEW", "float16-coordinate-types"));
    fresh_db()
}

fn kind_of(v: &Value) -> String {
    let Value::Vector { coord, .. } = v else {
        panic!("expected a vector, got {v:?}");
    };
    format!("{coord:?}")
}

fn content_of(fixture_name: &str) -> String {
    let text = String::from_utf8(fixture("valid", fixture_name)).unwrap();
    text.split("embedding`")
        .nth(1)
        .unwrap()
        .split('`')
        .next()
        .unwrap()
        .to_string()
}

#[test]
fn f16_and_bf16_embeddings_load_as_16_bit_vectors_and_export_unchanged() {
    for (name, kind) in [
        ("embedding16-f16", "Float16"),
        ("embedding16-bf16", "BFloat16"),
    ] {
        let (_db, mut c) = db();
        load_fixture(&mut c, "valid", name).unwrap_or_else(|f| panic!("{name}: {f:?}"));
        assert_eq!(kind_of(&node_prop(&mut c, "a", "e")), kind, "{name}");
        let text = export_str(&mut c).unwrap();
        let want = content_of(name);
        assert!(
            text.contains(&format!("embedding`{want}`")),
            "{name}: {text}"
        );
    }
}

#[test]
fn a_16_bit_vector_built_in_cypher_exports_and_reloads_equal() {
    for kind in ["FLOAT16", "BFLOAT16"] {
        let (_db, mut a) = db();
        run(
            &mut a,
            &format!("CREATE (:P {{e: vector([0.1, -2.5, 1000.0], 3, {kind})}})"),
        );
        let before = run(&mut a, "MATCH (n:P) RETURN n.e")[0][0].clone();
        let text = export_str(&mut a).unwrap();
        let (_db2, mut b) = db();
        load_str(&mut b, &text).unwrap();
        let after = run(&mut b, "MATCH (n:P) RETURN n.e")[0][0].clone();
        assert_eq!(before, after, "{kind}");
    }
}

/// Every finite bit pattern of a 16-bit format, as payloads of at most 4096 elements.
fn finite_chunks(dtype: EmbeddingDtype) -> Vec<Vec<u16>> {
    let finite = |bits: u16| match dtype {
        EmbeddingDtype::F16 => (bits >> 10) & 0x1f != 0x1f,
        _ => (bits >> 7) & 0xff != 0xff,
    };
    let all: Vec<u16> = (0..=u16::MAX).filter(|b| finite(*b)).collect();
    all.chunks(4096).map(|c| c.to_vec()).collect()
}

#[test]
fn every_finite_16_bit_pattern_survives_load_and_export_byte_for_byte() {
    for dtype in [EmbeddingDtype::F16, EmbeddingDtype::BF16] {
        let chunks = finite_chunks(dtype);
        let contents: Vec<String> = chunks
            .iter()
            .map(|c| {
                let bytes: Vec<u8> = c.iter().flat_map(|b| b.to_le_bytes()).collect();
                Embedding::new(dtype, c.len(), bytes)
                    .unwrap()
                    .to_normal_content()
            })
            .collect();
        let source: String = contents
            .iter()
            .map(|c| format!("(:P {{e: embedding`{c}`}})\n"))
            .collect();
        let (_db, mut c) = db();
        load_str(&mut c, &source).unwrap_or_else(|f| panic!("{dtype:?}: {:?}", f.diagnostics));
        let text = export_str(&mut c).unwrap();
        let mut got: Vec<String> = text
            .split("embedding`")
            .skip(1)
            .map(|r| r.split('`').next().unwrap().to_string())
            .collect();
        let mut want = contents.clone();
        got.sort();
        want.sort();
        assert_eq!(got.len(), want.len(), "{dtype:?}");
        assert!(
            got == want,
            "{dtype:?}: some pattern changed through Relate"
        );
    }
}
