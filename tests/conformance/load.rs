//! The `load` kind (ADR-015 section 2.2): a text that must load into a database made by
//! `start.cypher`, with the warnings and counts expected exactly and the graph afterwards equal to
//! `expected.gram`.
use super::case::Case;
use super::expected::{read_expected, Identity};
use super::{db, expect, graph, settings};

fn describe(ds: &[relate_gram::diagnostics::Diagnostic]) -> String {
    ds.iter()
        .map(|d| match d.span {
            Some(s) => format!("{} at {}:{}", d.code, s.line, s.column),
            None => d.code.to_string(),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Run a `load` case on its text, starting statements and expected graph.
pub fn run_load_text(
    case: &Case,
    text: &[u8],
    start: &str,
    expected_gram: &str,
) -> Result<(), String> {
    let load_settings = settings::load_settings(&case.settings)?;
    let identity = Identity {
        node_property: load_settings.node_id_property.clone(),
        relationship_property: load_settings.relationship_id_property.clone(),
    };
    let expected = read_expected(expected_gram, &identity)?;
    let (_db, mut conn) = db::fresh(start)?;
    let report = match relate_gram::load(&mut conn, text, &load_settings) {
        Ok(r) => r,
        Err(f) => {
            return Err(format!(
                "the text did not load: {}",
                describe(&f.diagnostics)
            ))
        }
    };
    let mut problems = Vec::new();
    let (errors, warnings) = expect::split(&report.diagnostics);
    if let Err(e) = expect::compare_diagnostics("Error", &case.errors(), &errors) {
        problems.push(e);
    }
    if let Err(e) = expect::compare_diagnostics("Warning", &case.warnings(), &warnings) {
        problems.push(e);
    }
    if let Some(counts) = case.counts() {
        if let Err(e) = expect::compare_counts(counts, &report.counts, None) {
            problems.push(e);
        }
    }
    let got = graph::dump_database(&mut conn, false)?;
    if got != expected {
        problems.push(graph::describe_difference(&expected, &got));
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("; "))
    }
}

/// Run a `load` case from its directory.
pub fn run_load(case: &Case) -> Result<(), String> {
    let read = |name: &str| std::fs::read(case.dir.join(name)).map_err(|e| format!("{name}: {e}"));
    let text = read("input.txt")?;
    let expected = String::from_utf8(read("expected.gram")?).map_err(|e| e.to_string())?;
    let start = std::fs::read_to_string(case.dir.join("start.cypher")).unwrap_or_default();
    run_load_text(case, &text, &start, &expected)
}
