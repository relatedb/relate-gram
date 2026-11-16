//! Runs the conformance collection (ADR-015, spec 006) with Relate's 16-bit preview flag unset.
//! `conformance16.rs` is the pass with the flag set.
#![allow(dead_code)]

#[path = "conformance/mod.rs"]
mod conformance;

use conformance::case::{read_case, Case, Kind};
use conformance::{collection, export, fix, load, reject};

fn run_kind(kind: Kind, run: impl Fn(&Case) -> Result<(), String>) {
    std::env::remove_var("RELATE_PREVIEW");
    let cases = collection::cases(kind);
    let mut failures = Vec::new();
    for (id, dir) in &cases {
        // Until the capability probes of T036 run, the 16-bit vector cases wait for the second pass.
        if read_case(dir, kind, id).is_ok_and(|c| c.requires.iter().any(|r| r == "vector-16-bit")) {
            continue;
        }
        match read_case(dir, kind, id).and_then(|c| run(&c)) {
            Ok(()) => {}
            Err(e) => failures.push(format!("{}/{id}: {e}", kind.dir())),
        }
    }
    println!("{} cases of kind {}", cases.len(), kind.dir());
    assert!(
        failures.is_empty(),
        "{} of {} {} cases failed:\n  {}",
        failures.len(),
        cases.len(),
        kind.dir(),
        failures.join("\n  ")
    );
}

#[test]
fn reject_cases() {
    run_kind(Kind::Reject, reject::run_reject);
}

#[test]
fn load_cases() {
    run_kind(Kind::Load, load::run_load);
}

#[test]
fn export_cases() {
    run_kind(Kind::Export, export::run_export);
}

#[test]
fn fix_cases() {
    run_kind(Kind::Fix, fix::run_fix);
}

#[test]
fn fixed_text_is_a_fixed_point() {
    run_kind(Kind::Fix, fix::run_fix_is_a_fixed_point);
}
