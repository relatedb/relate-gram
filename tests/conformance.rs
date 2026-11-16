//! Runs the conformance collection (ADR-015, spec 006) with Relate's 16-bit preview flag unset.
//! `conformance16.rs` is the pass with the flag set; together they cover every case.
#![allow(dead_code)]

#[path = "conformance/mod.rs"]
mod conformance;

use conformance::case::{read_case, Kind};
use conformance::{collection, fix, suite};

#[test]
fn collection_with_the_16_bit_flag_unset() {
    std::env::remove_var("RELATE_PREVIEW");
    suite::run_and_assert("flag-unset");
}

#[test]
fn fixed_text_is_a_fixed_point() {
    for (id, dir) in collection::cases(Kind::Fix) {
        let case = read_case(&dir, Kind::Fix, &id).unwrap();
        fix::run_fix_is_a_fixed_point(&case).unwrap_or_else(|e| panic!("fix/{id}: {e}"));
    }
}
