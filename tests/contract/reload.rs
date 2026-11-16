//! Story 3 (ADR-001 section 2.7).
use super::common::*;
use relate::Value;

fn count(c: &mut relate::Connection, q: &str) -> Value {
    run(c, q)[0][0].clone()
}

#[test]
fn duplicates_the_file_does_not_touch_are_not_an_error() {
    let (_db, mut c) = fresh_db();
    run(&mut c, "CREATE (:L {`@id`: 'a'}), (:L {`@id`: 'a'})");
    load_str(&mut c, "(b:L)").unwrap();
    assert_eq!(count(&mut c, "MATCH (n) RETURN count(n)"), Value::Int(3));
}
