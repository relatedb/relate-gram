//! ADR-014 section 2.2: without Relate's preview flag a 16-bit embedding stops the load before
//! anything is written, with one error at each such value.
mod common;
use common::*;
use relate::Value;
use relate_gram::settings::LoadSettings;

fn off() -> (relate::Database, relate::Connection) {
    std::env::remove_var("RELATE_PREVIEW");
    fresh_db()
}

fn content_of(name: &str) -> String {
    let text = String::from_utf8(fixture("valid", name)).unwrap();
    text.split("embedding`")
        .nth(1)
        .unwrap()
        .split('`')
        .next()
        .unwrap()
        .to_string()
}

fn mixed_file() -> String {
    let f16 = content_of("embedding16-f16");
    let bf16 = content_of("embedding16-bf16");
    let f32c = content_of("embedding-f32");
    format!(
        "(a {{e: embedding`{f16}`}})\n(b {{e: embedding`{f32c}`, g: embedding`{f16}`}})\n(c {{e: embedding`{bf16}`}})\n"
    )
}

#[test]
fn sixteen_bit_embeddings_without_the_flag_fail_before_any_write_with_one_error_each() {
    let (_db, mut c) = off();
    let src = mixed_file();
    let f = relate_gram::load(&mut c, src.as_bytes(), &LoadSettings::default()).unwrap_err();
    assert_eq!(
        codes(&f.diagnostics),
        vec!["value.tag-content"; 3],
        "{:?}",
        f.diagnostics
    );
    for d in &f.diagnostics {
        assert!(
            d.message
                .contains("RELATE_PREVIEW=float16-coordinate-types"),
            "{}",
            d.message
        );
        let at = d.span.unwrap().start;
        assert!(
            src[at..].starts_with("embedding`"),
            "error not at an embedding: {:?}",
            d.span
        );
    }
    let rows = run(&mut c, "MATCH (n) RETURN count(n)");
    assert_eq!(rows[0][0], Value::Int(0), "nothing was written");
}

#[test]
fn only_f32_and_f64_embeddings_load_without_the_flag() {
    let (_db, mut c) = off();
    load_fixture(&mut c, "valid", "embedding-f32").unwrap();
    load_fixture(&mut c, "valid", "embedding-f64").unwrap();
}

#[test]
fn check_passes_a_16_bit_embedding_because_it_cannot_know_the_flag() {
    let (_db, _c) = off();
    let r = relate_gram::check(mixed_file().as_bytes(), &LoadSettings::default());
    assert!(!r.has_errors(), "{:?}", r.diagnostics);
}
