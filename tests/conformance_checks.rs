//! The checks of the collection itself (ADR-015 section 2.8, spec 006 User Story 6): the files lint,
//! the names are in the vocabulary, the versions are well formed, identifiers are unique, the
//! attributes keep the byte strings intact, and every diagnostic code has a case. Each fault is
//! shown to fail the check on a temporary copy of the collection.
#![allow(dead_code)]

#[path = "conformance/mod.rs"]
mod conformance;

use conformance::checks::{check_collection, has_gram_tool};
use conformance::collection::root;
use relate_gram::diagnostics::code;
use std::path::{Path, PathBuf};

fn require_lint() -> bool {
    std::env::var("CONFORMANCE_REQUIRE_LINT").is_ok_and(|v| v == "1")
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap().flatten() {
        let (src, dst) = (e.path(), to.join(e.file_name()));
        if src.is_dir() {
            copy_dir(&src, &dst);
        } else {
            std::fs::copy(&src, &dst).unwrap();
        }
    }
}

/// A copy of the collection, with its `.gitattributes` beside it, under a directory of its own.
fn copy(name: &str) -> PathBuf {
    let base =
        std::env::temp_dir().join(format!("conformance-check-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    copy_dir(&root(), &base.join("conformance"));
    std::fs::copy(
        root().parent().unwrap().join(".gitattributes"),
        base.join(".gitattributes"),
    )
    .unwrap();
    base
}

fn problems(base: &Path) -> Vec<String> {
    check_collection(&base.join("conformance"), false, code::ALL)
}

fn expect_problem(name: &str, fault: impl Fn(&Path), needle: &str) {
    let base = copy(name);
    assert!(problems(&base).is_empty(), "{name}: the copy starts clean");
    fault(&base.join("conformance"));
    let got = problems(&base);
    let _ = std::fs::remove_dir_all(&base);
    assert!(
        got.iter().any(|p| p.contains(needle)),
        "{name}: expected a problem containing `{needle}`, got {got:?}"
    );
}

#[test]
fn the_collection_passes_its_checks() {
    let found = check_collection(&root(), require_lint(), code::ALL);
    assert!(found.is_empty(), "{}", found.join("\n"));
}

#[test]
fn a_lint_error_in_a_case_file_fails() {
    if !has_gram_tool() {
        eprintln!("skipped: the `gram` tool is not installed (cargo install gram-data --locked --version 0.7.0)");
        assert!(
            !require_lint(),
            "CONFORMANCE_REQUIRE_LINT is set but `gram` is missing"
        );
        return;
    }
    let base = copy("lint");
    let broken = base.join("conformance/reject/key-reserved/case.gram");
    std::fs::write(&broken, "{requires: []\n(:Error {code: ").unwrap();
    let got = check_collection(&base.join("conformance"), true, code::ALL);
    let _ = std::fs::remove_dir_all(&base);
    assert!(
        got.iter()
            .any(|p| p.contains("lint") && p.contains("key-reserved")),
        "{got:?}"
    );
}

#[test]
fn a_label_or_capability_not_in_the_vocabulary_fails() {
    expect_problem(
        "label",
        |c| {
            let p = c.join("reject/key-reserved/case.gram");
            let s = std::fs::read_to_string(&p)
                .unwrap()
                .replace(":Error", ":Eror");
            std::fs::write(p, s).unwrap();
        },
        "Eror",
    );
    expect_problem(
        "capability",
        |c| {
            let p = c.join("load/typed-date/case.gram");
            let s = std::fs::read_to_string(&p)
                .unwrap()
                .replace("temporal-values", "temporal");
            std::fs::write(p, s).unwrap();
        },
        "temporal",
    );
}

#[test]
fn a_missing_or_malformed_version_fails() {
    expect_problem(
        "no-version",
        |c| std::fs::remove_file(c.join("VERSION")).unwrap(),
        "VERSION",
    );
    expect_problem(
        "bad-version",
        |c| std::fs::write(c.join("VERSION"), "1.0\n").unwrap(),
        "MAJOR.MINOR.PATCH",
    );
    expect_problem(
        "no-grammar-version",
        |c| std::fs::remove_file(c.join("GRAMMAR_VERSION")).unwrap(),
        "GRAMMAR_VERSION",
    );
}

#[test]
fn a_duplicate_case_identifier_in_one_kind_fails() {
    expect_problem(
        "duplicate",
        |c| {
            copy_dir(
                &c.join("export/empty-database"),
                &c.join("export/empty_database"),
            )
        },
        "duplicate",
    );
}

#[test]
fn a_code_with_no_case_and_no_reason_fails() {
    let base = copy("code");
    let got = check_collection(&base.join("conformance"), false, &["no.such-code"]);
    let _ = std::fs::remove_dir_all(&base);
    assert!(got.iter().any(|p| p.contains("no.such-code")), "{got:?}");
}

#[test]
fn a_missing_gitattributes_rule_fails() {
    let base = copy("attributes");
    std::fs::write(base.join(".gitattributes"), "").unwrap();
    let got = problems(&base);
    let _ = std::fs::remove_dir_all(&base);
    assert!(got.iter().any(|p| p.contains(".gitattributes")), "{got:?}");
}
