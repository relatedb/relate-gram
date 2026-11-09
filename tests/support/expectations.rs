//! The table in `corpus_expectations.toml`.
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Expectation {
    pub file: String,
    pub name: String,
    pub expect: String,
    #[serde(default)]
    pub codes: Vec<String>,
}

#[derive(Deserialize)]
struct Table {
    case: Vec<Expectation>,
}

pub fn read() -> Vec<Expectation> {
    let path = format!(
        "{}/tests/support/corpus_expectations.toml",
        env!("CARGO_MANIFEST_DIR")
    );
    let text = std::fs::read_to_string(&path).unwrap();
    toml::from_str::<Table>(&text).unwrap().case
}
