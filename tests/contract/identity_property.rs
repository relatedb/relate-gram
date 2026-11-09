//! Story 3, scenario 4 (ADR-001 section 2.10).
use super::common::*;
use relate::Value;
use relate_gram::diagnostics::code;
use relate_gram::settings::LoadSettings;
fn nodes_by(prop: &str) -> LoadSettings {
    LoadSettings {
        node_id_property: Some(prop.to_string()),
        ..LoadSettings::default()
    }
}

fn rels_by(prop: &str) -> LoadSettings {
    LoadSettings {
        relationship_id_property: Some(prop.to_string()),
        ..LoadSettings::default()
    }
}

fn count(c: &mut relate::Connection, q: &str) -> Value {
    run(c, q)[0][0].clone()
}

#[test]
fn elements_match_on_the_property_and_no_id_is_written() {
    let (_db, mut c) = fresh_db();
    let st = nodes_by("email");
    relate_gram::load(&mut c, b"(a:P {email: \"x@y\"})", &st).unwrap();
    // a different gram identifier, the same identity value: the same node
    let r = relate_gram::load(&mut c, b"(b:P {email: \"x@y\", age: 2})", &st).unwrap();
    assert_eq!(count(&mut c, "MATCH (n:P) RETURN count(n)"), Value::Int(1));
    assert_eq!(count(&mut c, "MATCH (n:P) RETURN n.age"), Value::Int(2));
    assert_eq!(count(&mut c, "MATCH (n:P) RETURN n.`@id`"), Value::Null);
    assert_eq!(r.counts.nodes_created, 0);
    assert_eq!(r.counts.nodes_updated, 1);
}

#[test]
fn the_integer_and_the_string_with_the_same_text_are_one_identity() {
    let (_db, mut c) = fresh_db();
    run(&mut c, "CREATE (:P {uid: '42'})");
    relate_gram::load(&mut c, b"(a:P {uid: 42})", &nodes_by("uid")).unwrap();
    assert_eq!(count(&mut c, "MATCH (n:P) RETURN count(n)"), Value::Int(1));
    // the file's value wins, so the property is now the integer
    assert_eq!(count(&mut c, "MATCH (n:P) RETURN n.uid"), Value::Int(42));
}

#[test]
fn a_definition_without_the_property_is_anonymous_and_reported() {
    let (_db, mut c) = fresh_db();
    let r = relate_gram::load(&mut c, b"(a:P {name: \"n\"})", &nodes_by("email")).unwrap();
    let w: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == code::ID_PROPERTY_MISSING)
        .collect();
    assert_eq!(w.len(), 1, "{:?}", r.diagnostics);
    assert_eq!(w[0].span.unwrap().line, 1);
    assert_eq!(r.counts.elements_without_identifier, 1);
    assert_eq!(count(&mut c, "MATCH (n:P) RETURN n.`@id`"), Value::Null);
    // loaded again it is created again
    relate_gram::load(&mut c, b"(a:P {name: \"n\"})", &nodes_by("email")).unwrap();
    assert_eq!(count(&mut c, "MATCH (n:P) RETURN count(n)"), Value::Int(2));
}

#[test]
fn stored_elements_with_equal_text_are_reported_when_the_file_matches_them() {
    let (_db, mut c) = fresh_db();
    run(&mut c, "CREATE (:P {uid: '1'}), (:P {uid: 1})");
    let f = relate_gram::load(&mut c, b"(a:P {uid: 1})", &nodes_by("uid")).unwrap_err();
    assert_eq!(f.diagnostics[0].code, code::MATCH_DB_DUPLICATE);
    assert_eq!(count(&mut c, "MATCH (n:P) RETURN count(n)"), Value::Int(2));
}

#[test]
fn two_file_elements_with_one_identity_value_are_an_error() {
    let (_db, mut c) = fresh_db();
    let f = relate_gram::load(
        &mut c,
        b"(a:P {email: \"x\"})\n(b:P {email: \"x\"})",
        &nodes_by("email"),
    )
    .unwrap_err();
    assert_eq!(f.diagnostics[0].code, code::ID_DUPLICATE_DEFINITION);
    assert_eq!(f.diagnostics[0].span.unwrap().line, 2);
    assert_eq!(f.diagnostics[0].related[0].1.line, 1);
}

#[test]
fn relationships_match_on_their_own_property() {
    let (_db, mut c) = fresh_db();
    let st = rels_by("rid");
    let src = b"(a:N)-[r:T {rid: \"z\", w: 1}]->(b:N)";
    relate_gram::load(&mut c, src, &st).unwrap();
    // nodes still use @id, so they match; the relationship matches on `rid`
    relate_gram::load(&mut c, b"(a:N)-[q:T {rid: \"z\", w: 2}]->(b:N)", &st).unwrap();
    assert_eq!(
        count(&mut c, "MATCH ()-[r:T]->() RETURN count(r)"),
        Value::Int(1)
    );
    assert_eq!(
        count(&mut c, "MATCH ()-[r:T]->() RETURN r.w"),
        Value::Int(2)
    );
    assert_eq!(
        count(&mut c, "MATCH ()-[r:T]->() RETURN r.`@id`"),
        Value::Null
    );
}
