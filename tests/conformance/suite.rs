//! Running the whole collection in one pass (ADR-015 section 2.6): probe the capabilities, run each
//! applicable case, decide its status against the known failures, write the results file and fail on
//! any `failed`. A pass is one state of Relate's 16-bit preview flag, which the caller sets before
//! calling this, because the flag is process-wide.
use super::capabilities::{applicable, apply_declared, operations, probe};
use super::case::{read_case, Case, Kind};
use super::results::{decide, known_failures, results_text, Status};
use super::{collection, export, fix, load, reject};
use std::path::PathBuf;

const KINDS: [Kind; 4] = [Kind::Reject, Kind::Load, Kind::Export, Kind::Fix];

fn run_case(case: &Case) -> Result<(), String> {
    match case.kind {
        Kind::Reject => reject::run_reject(case),
        Kind::Load => load::run_load(case),
        Kind::Export => export::run_export(case),
        Kind::Fix => fix::run_fix(case),
    }
}

/// Where the results of a pass are written: `target/conformance/<pass>.gram`.
pub fn results_path(pass: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("conformance")
        .join(format!("{pass}.gram"))
}

type PassResult = (Vec<(String, Status)>, Vec<String>);

/// Run every case once and write the results file. Returns the statuses and the failure messages.
pub fn run_pass(pass: &str) -> Result<PassResult, String> {
    let root = collection::root();
    let read =
        |name: &str| std::fs::read_to_string(root.join(name)).map_err(|e| format!("{name}: {e}"));
    let mut present = probe(&read("capabilities.gram")?)?;
    if let Ok(declared) = std::fs::read_to_string(root.join("declared-capabilities.gram")) {
        present = apply_declared(&present, &declared)?;
    }
    let known = known_failures(&read("known-failures.gram")?)?;
    let version = read("VERSION")?.trim().to_string();
    let (mut rows, mut failures) = (Vec::new(), Vec::new());
    for kind in KINDS {
        for (id, dir) in collection::cases(kind) {
            let name = format!("{}/{id}", kind.dir());
            let case = read_case(&dir, kind, &id)?;
            let applies = applicable(&case.requires, &present)
                && operations(&case).iter().all(|o| present.contains(*o));
            let outcome = applies.then(|| run_case(&case));
            let (status, why) = decide(applies, outcome, known.get(&name));
            if let Some(why) = why {
                failures.push(format!("{name}: {why}"));
            }
            rows.push((name, status));
        }
    }
    let path = results_path(pass);
    std::fs::create_dir_all(path.parent().expect("a parent directory"))
        .map_err(|e| e.to_string())?;
    std::fs::write(&path, results_text(&rows, &version)).map_err(|e| e.to_string())?;
    Ok((rows, failures))
}

/// Run a pass and fail the test on any failed case, printing the counts per status.
pub fn run_and_assert(pass: &str) {
    let (rows, failures) = run_pass(pass).expect("the pass could not run");
    let count = |s: Status| rows.iter().filter(|(_, st)| *st == s).count();
    println!(
        "{pass}: {} cases, {} passed, {} failed, {} skipped, {} not applicable",
        rows.len(),
        count(Status::Passed),
        count(Status::Failed),
        count(Status::Skipped),
        count(Status::NotApplicable)
    );
    assert!(
        failures.is_empty(),
        "{} case(s) failed:\n  {}",
        failures.len(),
        failures.join("\n  ")
    );
}
