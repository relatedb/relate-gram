//! The second pass of the conformance collection (ADR-015 section 2.6): Relate's 16-bit preview
//! flag is set before the first connection, so the cases that require `vector-16-bit` run here and
//! the ones that require its absence are not applicable.
#![allow(dead_code)]

#[path = "conformance/mod.rs"]
mod conformance;

use conformance::suite;

#[test]
fn collection_with_the_16_bit_flag_set() {
    std::env::set_var("RELATE_PREVIEW", "float16-coordinate-types");
    suite::run_and_assert("flag-set");
}
