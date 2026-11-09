//! Error positions (Constitution VI, FR-015, SC-002): every fixture under
//! `tests/fixtures/invalid/` is loaded into a database that already holds
//! known content. Each expected diagnostic must be reported with its code,
//! line, column and byte offset, nothing else may be reported as an error,
//! and the database must be unchanged afterwards.
#[path = "../common/mod.rs"]
mod common;

use common::*;
use relate_gram::diagnostics::code;
use std::collections::BTreeSet;
use std::path::PathBuf;

fn fixtures() -> Vec<PathBuf> {
    let dir = format!("{}/tests/fixtures/invalid", env!("CARGO_MANIFEST_DIR"));
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "gram"))
        .collect();
    v.sort();
    v
}

fn expectations(path: &std::path::Path) -> Vec<(String, usize, usize, usize)> {
    let text = std::fs::read_to_string(path.with_extension("expect"))
        .unwrap_or_else(|e| panic!("{}: missing .expect: {e}", path.display()));
    text.lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .map(|l| {
            let p: Vec<&str> = l.split_whitespace().collect();
            assert_eq!(p.len(), 4, "bad .expect line {l:?}");
            (
                p[0].to_string(),
                p[1].parse().unwrap(),
                p[2].parse().unwrap(),
                p[3].parse().unwrap(),
            )
        })
        .collect()
}

#[test]
fn every_fixture_is_rejected_at_the_expected_position_and_changes_nothing() {
    let all = fixtures();
    assert!(all.len() > 30, "fixtures were not found");
    for path in all {
        let (_db, mut c) = fresh_db();
        run(&mut c, "CREATE (:Existing {k: 1})-[:R]->(:Existing {k: 2})");
        let before = dump_graph(&mut c);

        let bytes = std::fs::read(&path).unwrap();
        let failure = relate_gram::load(&mut c, &bytes, &Default::default())
            .err()
            .unwrap_or_else(|| panic!("{} should fail", path.display()));

        let mut got: Vec<(String, usize, usize, usize)> = failure
            .diagnostics
            .iter()
            .filter(|d| d.is_error())
            .map(|d| {
                let s = d
                    .span
                    .unwrap_or_else(|| panic!("{}: {} has no span", path.display(), d.code));
                (d.code.to_string(), s.line, s.column, s.start)
            })
            .collect();
        let mut want = expectations(&path);
        got.sort();
        want.sort();
        assert_eq!(got, want, "{}", path.display());

        assert_eq!(
            dump_graph(&mut c),
            before,
            "{} must leave the database unchanged",
            path.display()
        );
        assert!(
            !c.in_transaction(),
            "{} must not leave a transaction open",
            path.display()
        );
    }
}

/// The error codes a fixture can trigger. The others are tested elsewhere:
/// `match.*` and `id.property-missing` in the reload tests, the warnings and
/// `load.in-transaction` and `load.database-error` in `load_errors`, and
/// `construct.unsupported` by a unit test in the plan module.
const FIXTURE_CODES: &[&str] = &[
    code::INPUT_ENCODING,
    code::PARSE_ERROR,
    code::PARSE_MISSING,
    code::CONSTRUCT_ANNOTATION,
    code::CONSTRUCT_SUBJECT_PATTERN,
    code::CONSTRUCT_DOUBLE_COLON,
    code::REL_UNDIRECTED,
    code::REL_BIDIRECTIONAL,
    code::REL_ARROW_FAMILY,
    code::REL_NO_TYPE,
    code::REL_MANY_LABELS,
    code::VALUE_RANGE,
    code::VALUE_MEASUREMENT,
    code::VALUE_MAP,
    code::VALUE_OCTAL,
    code::VALUE_INTEGER_RANGE,
    code::VALUE_DECIMAL,
    code::VALUE_ARRAY_MIXED,
    code::KEY_DUPLICATE,
    code::KEY_EMPTY,
    code::KEY_RESERVED,
    code::ID_KIND_CONFLICT,
    code::ID_DUPLICATE_DEFINITION,
    code::ID_ENDPOINTS_DIFFER,
    code::ID_REL_NEVER_DEFINED,
    code::FILE_KIND,
];

#[test]
fn every_fixture_testable_code_has_a_fixture() {
    let covered: BTreeSet<String> = fixtures()
        .iter()
        .flat_map(|p| expectations(p))
        .map(|e| e.0)
        .collect();
    for c in FIXTURE_CODES {
        assert!(covered.contains(*c), "no fixture expects {c}");
    }
    for c in &covered {
        assert!(code::ALL.contains(&c.as_str()), "{c} is not a known code");
    }
}
