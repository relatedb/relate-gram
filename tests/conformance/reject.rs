//! The `reject` kind (ADR-015 section 2.2): a text that must fail, with the errors and warnings
//! expected exactly, the starting graph unchanged, and the same errors without a database.
use super::case::Case;
use super::{collection, db, expect, graph, settings};
use relate_gram::LoadCounts;

fn zero_counts() -> LoadCounts {
    LoadCounts {
        nodes_created: 0,
        nodes_updated: 0,
        relationships_created: 0,
        relationships_updated: 0,
        elements_without_identifier: 0,
        reference_only_nodes: 0,
        tags_dropped: 0,
    }
}

/// Run a `reject` case on its text and starting statements.
pub fn run_reject_text(case: &Case, text: &[u8], start: &str) -> Result<(), String> {
    let settings = settings::load_settings(&case.settings)?;
    let (_db, mut conn) = db::fresh(start)?;
    let before = graph::dump_database(&mut conn, false)?;
    if case.transaction {
        conn.begin().map_err(|e| e.to_string())?;
    }
    let result = relate_gram::load(&mut conn, text, &settings);
    if case.transaction {
        let _ = conn.rollback();
    }
    let failure = match result {
        Ok(_) => return Err("the text loaded; a `reject` case's text should be rejected".into()),
        Err(f) => f,
    };
    let (errors, warnings) = expect::split(&failure.diagnostics);
    let mut problems = Vec::new();
    if let Err(e) = expect::compare_diagnostics("Error", &case.errors(), &errors) {
        problems.push(e);
    }
    if let Err(e) = expect::compare_diagnostics("Warning", &case.warnings(), &warnings) {
        problems.push(e);
    }
    if let Some(counts) = case.counts() {
        if let Err(e) = expect::compare_counts(counts, &zero_counts(), Some(failure.total)) {
            problems.push(e);
        }
    }
    let after = graph::dump_database(&mut conn, false)?;
    if after != before {
        problems.push("the starting graph changed during a failed load".into());
    }
    // The same errors without a database. A transaction has no meaning there, and `check` does not
    // apply the cap on reported errors, so a case that sets `maxDiagnostics` compares `load` only.
    // A `match.*` error is found by comparing with stored elements, so `check` cannot report it.
    let needs_database = case.errors().iter().any(|e| e.code.starts_with("match."));
    if !case.transaction && !needs_database && !case.settings.contains_key("maxDiagnostics") {
        let checked = relate_gram::check(text, &settings);
        let (check_errors, _) = expect::split(&checked.diagnostics);
        if let Err(e) = expect::compare_diagnostics("check Error", &case.errors(), &check_errors) {
            problems.push(e);
        }
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("; "))
    }
}

/// Run a `reject` case from its directory.
pub fn run_reject(case: &Case) -> Result<(), String> {
    let text = std::fs::read(case.dir.join("input.txt"))
        .or_else(|_| std::fs::read(case.dir.join("input.bin")))
        .map_err(|e| format!("input: {e}"))?;
    let start = match std::fs::read_to_string(case.dir.join("start.cypher")) {
        Ok(s) => s,
        Err(_) => collection::standard_start(),
    };
    run_reject_text(case, &text, &start)
}
