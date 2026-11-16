//! Every error code that a `reject` case can trigger has a case (until the collection's own check,
//! spec 006 task T040, takes this over). The positions and the unchanged-database check of the
//! former per-fixture test are now the `reject` runner of the conformance collection (ADR-015).
use relate_gram::diagnostics::code;
use std::collections::BTreeSet;
use std::path::PathBuf;

fn fixtures() -> Vec<PathBuf> {
    let dir = format!("{}/conformance/reject", env!("CARGO_MANIFEST_DIR"));
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path().join("case.gram"))
        .collect();
    v.sort();
    v
}

/// The codes of the `Error` nodes of a `case.gram`.
fn expectations(path: &std::path::Path) -> Vec<(String,)> {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    text.lines()
        .filter(|l| l.starts_with("(:Error {code: \""))
        .map(|l| {
            (l["(:Error {code: \"".len()..]
                .split('"')
                .next()
                .unwrap()
                .to_string(),)
        })
        .collect()
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
