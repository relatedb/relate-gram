//! The `fix` kind (ADR-015 section 2.2): the fixer run on `input.txt` with the case's settings. The
//! output is compared byte for byte with `fixed.txt`; `Applied` nodes give the count of each code
//! corrected, and `Remaining` nodes the errors left in the output, with their positions in it.
//!
//! The settings are the fixer's options. A key that only the loader reads (`tags`, which makes a
//! tag an error to fix) is passed on to the loader settings the fixer checks the text with.
use super::case::{Case, Diag, Setting};
use super::{expect, export::first_difference, settings};
use std::collections::BTreeMap;

const FIX_KEYS: [&str; 4] = ["relType", "direction", "octal", "annotate"];

/// Run a `fix` case on its input and expected text.
pub fn run_fix_text(case: &Case, input: &[u8], fixed: &[u8]) -> Result<(), String> {
    let (fix_part, load_part): (BTreeMap<String, Setting>, BTreeMap<String, Setting>) = case
        .settings
        .clone()
        .into_iter()
        .partition(|(k, _)| FIX_KEYS.contains(&k.as_str()));
    let options = settings::fix_options(&fix_part)?;
    let load_settings = settings::load_settings(&load_part)?;
    let out = relate_gram::fix::fix(input, &load_settings, &options)
        .map_err(|f| format!("the fixer refused: {f}"))?;
    let mut problems = Vec::new();
    if out.text != fixed {
        problems.push(first_difference(fixed, &out.text));
    }
    let mut applied: BTreeMap<String, i64> = BTreeMap::new();
    for f in &out.applied {
        *applied.entry(f.code.to_string()).or_default() += 1;
    }
    let mut want: BTreeMap<String, i64> = BTreeMap::new();
    for (code, count) in case.applied() {
        *want.entry(code).or_default() += count;
    }
    if applied != want {
        problems.push(format!("Applied: expected {want:?}, got {applied:?}"));
    }
    let (errors, _) = expect::split(&out.remaining);
    let errors: Vec<Diag> = errors;
    if let Err(e) = expect::compare_diagnostics("Remaining", &case.remaining(), &errors) {
        problems.push(e);
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("; "))
    }
}

/// Run a `fix` case from its directory.
pub fn run_fix(case: &Case) -> Result<(), String> {
    let read = |name: &str| std::fs::read(case.dir.join(name)).map_err(|e| format!("{name}: {e}"));
    run_fix_text(case, &read("input.txt")?, &read("fixed.txt")?)
}

/// Fixing a case's `fixed.txt` again, with the same settings, changes nothing and applies nothing
/// (ADR-010: the fixer is idempotent). Kept beside the collection because the examples it replaced
/// each checked it.
pub fn run_fix_is_a_fixed_point(case: &Case) -> Result<(), String> {
    let (fix_part, load_part): (BTreeMap<String, Setting>, BTreeMap<String, Setting>) = case
        .settings
        .clone()
        .into_iter()
        .partition(|(k, _)| FIX_KEYS.contains(&k.as_str()));
    let options = settings::fix_options(&fix_part)?;
    let load_settings = settings::load_settings(&load_part)?;
    let fixed = std::fs::read(case.dir.join("fixed.txt")).map_err(|e| e.to_string())?;
    let again = relate_gram::fix::fix(&fixed, &load_settings, &options)
        .map_err(|f| format!("the fixer refused its own output: {f}"))?;
    if again.text != fixed || !again.applied.is_empty() {
        return Err("fixing fixed.txt again changed it".into());
    }
    Ok(())
}
