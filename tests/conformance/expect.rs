//! Matching what the library reported against what a case expects (ADR-015 section 2.4):
//! diagnostics exhaustively by code and position, counts over the keys listed. Message wording
//! and related positions are not compared.
use super::case::Diag;
use relate_gram::diagnostics::{Diagnostic, Severity};
use relate_gram::LoadCounts;
use std::collections::BTreeMap;

fn describe(d: &Diag) -> String {
    match d.position {
        Some((l, c, o)) => format!("{} at {l}:{c} (offset {o})", d.code),
        None => format!("{} (no position)", d.code),
    }
}

/// Split the library's diagnostics into errors and warnings as expectations.
pub fn split(ds: &[Diagnostic]) -> (Vec<Diag>, Vec<Diag>) {
    let (mut errors, mut warnings) = (Vec::new(), Vec::new());
    for d in ds {
        let diag = Diag {
            code: d.code.to_string(),
            position: d.span.map(|s| (s.line, s.column, s.start)),
        };
        match d.severity {
            Severity::Error => errors.push(diag),
            Severity::Warning => warnings.push(diag),
        }
    }
    (errors, warnings)
}

/// Compare an expected list with the reported list, exhaustively and in any order. `what` is the
/// label (`Error`, `Warning`, `Remaining`) used in the message.
pub fn compare_diagnostics(what: &str, expected: &[Diag], got: &[Diag]) -> Result<(), String> {
    let mut missing: Vec<Diag> = expected.to_vec();
    let mut unexpected: Vec<Diag> = Vec::new();
    for g in got {
        if let Some(i) = missing.iter().position(|e| e == g) {
            missing.remove(i);
        } else {
            unexpected.push(g.clone());
        }
    }
    if missing.is_empty() && unexpected.is_empty() {
        return Ok(());
    }
    let mut lines = Vec::new();
    // pair a missing and an unexpected diagnostic of one code: the position is what differs
    let mut i = 0;
    while i < missing.len() {
        if let Some(j) = unexpected.iter().position(|u| u.code == missing[i].code) {
            let (m, u) = (missing.remove(i), unexpected.remove(j));
            let part = match (m.position, u.position) {
                (Some((l1, c1, o1)), Some((l2, c2, o2))) => {
                    let mut diffs = Vec::new();
                    if l1 != l2 {
                        diffs.push("line");
                    }
                    if c1 != c2 {
                        diffs.push("column");
                    }
                    if o1 != o2 {
                        diffs.push("offset");
                    }
                    diffs.join(" and ")
                }
                _ => "position".to_string(),
            };
            lines.push(format!(
                "{what} {}: the {part} differs; expected {}, got {}",
                m.code,
                describe(&m),
                describe(&u)
            ));
        } else {
            i += 1;
        }
    }
    for m in &missing {
        lines.push(format!("{what} missing: expected {}", describe(m)));
    }
    for u in &unexpected {
        lines.push(format!("{what} unexpected: got {}", describe(u)));
    }
    Err(lines.join("; "))
}

/// Compare a `Counts` node with the load's counts, over the keys the node lists. `total` is the
/// number of errors found before the cap, known only for a failed load.
pub fn compare_counts(
    expected: &BTreeMap<String, i64>,
    got: &LoadCounts,
    total: Option<usize>,
) -> Result<(), String> {
    let mut bad = Vec::new();
    for (k, want) in expected {
        let have: Option<i64> = match k.as_str() {
            "nodesCreated" => Some(got.nodes_created as i64),
            "nodesUpdated" => Some(got.nodes_updated as i64),
            "relationshipsCreated" => Some(got.relationships_created as i64),
            "relationshipsUpdated" => Some(got.relationships_updated as i64),
            "elementsWithoutIdentifier" => Some(got.elements_without_identifier as i64),
            "referenceOnlyNodes" => Some(got.reference_only_nodes as i64),
            "tagsDropped" => Some(got.tags_dropped as i64),
            "total" => total.map(|t| t as i64),
            _ => None,
        };
        match have {
            Some(h) if h == *want => {}
            Some(h) => bad.push(format!("Counts {k}: expected {want}, got {h}")),
            None => bad.push(format!("Counts {k}: no value to compare")),
        }
    }
    if bad.is_empty() {
        Ok(())
    } else {
        Err(bad.join("; "))
    }
}
