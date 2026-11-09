//! Story 5: generated graphs survive export and load (Constitution IV).
use super::common::*;
use proptest::prelude::*;
use relate::Value;
use std::sync::Arc;

fn text() -> impl Strategy<Value = String> {
    prop_oneof![
        "[^\\x00]{0,10}",
        // text built from the characters that need care: slashes (a quoted text
        // that starts with `//` is read as a comment), blanks, quotes, backslashes
        "[ /*\\\\a\"'`\\t]{0,10}",
    ]
}

fn name() -> impl Strategy<Value = String> {
    // keys and labels: any non-empty text without NUL, and a key cannot start with `@`
    prop_oneof!["[^\\x00@][^\\x00]{0,7}", "[ /*\\\\a\"'`\\t]{1,6}"]
}

fn scalar_list() -> impl Strategy<Value = Value> {
    prop_oneof![
        prop::collection::vec(any::<i64>().prop_map(Value::Int), 1..4),
        prop::collection::vec(
            any::<f64>()
                .prop_filter("finite", |f| f.is_finite())
                .prop_map(Value::Float),
            1..4
        ),
        prop::collection::vec(any::<bool>().prop_map(Value::Bool), 1..4),
        prop::collection::vec(
            text().prop_map(|s| Value::String(Arc::from(s.as_str()))),
            1..4
        ),
    ]
    .prop_map(|v| Value::List(Arc::new(v)))
}

fn value() -> impl Strategy<Value = Value> {
    prop_oneof![
        any::<i64>().prop_map(Value::Int),
        any::<f64>()
            .prop_filter("finite", |f| f.is_finite())
            .prop_map(Value::Float),
        any::<bool>().prop_map(Value::Bool),
        text().prop_map(|s| Value::String(Arc::from(s.as_str()))),
        scalar_list(),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    #[test]
    fn a_node_with_any_labels_keys_values_and_identifier_round_trips(
        id in prop_oneof!["[^\\x00]{1,8}", "[ /*\\\\a\"'`\\t]{1,6}"],
        labels in prop::collection::btree_set(name(), 0..3),
        props in prop::collection::btree_map(name(), value(), 0..5),
    ) {
        let (_a, mut first) = fresh_db();
        let mut all: Vec<(&str, Value)> = props.iter().map(|(k, v)| (k.as_str(), v.clone())).collect();
        all.push(("@id", Value::String(Arc::from(id.as_str()))));
        let label_text: String = labels
            .iter()
            .map(|l| format!(":`{}`", l.replace('`', "``")))
            .collect();
        create_node(&mut first, &label_text, all);

        let text = export_str(&mut first).unwrap_or_else(|f| panic!("export failed: {f:?}"));
        let (_b, mut second) = fresh_db();
        load_str(&mut second, &text)
            .unwrap_or_else(|f| panic!("reload failed: {:?}\n{text}", f.diagnostics));
        prop_assert_eq!(dump_graph(&mut first), dump_graph(&mut second), "{}", text);
    }

    #[test]
    fn identifiers_of_every_shape_survive(ids in prop::collection::btree_set(
        prop_oneof!["[^\\x00]{1,6}", "[ /*\\\\a\"'`\\t]{1,6}"],
        2..4
    )) {
        let (_a, mut first) = fresh_db();
        for id in &ids {
            create_node(&mut first, ":N", vec![("@id", Value::String(Arc::from(id.as_str())))]);
        }
        let text = export_str(&mut first).unwrap();
        let (_b, mut second) = fresh_db();
        load_str(&mut second, &text).unwrap_or_else(|f| panic!("{:?}\n{text}", f.diagnostics));
        prop_assert_eq!(dump_graph(&mut first), dump_graph(&mut second));
    }
}
