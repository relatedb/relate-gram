//! The `relate-gram` command.
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_relate-gram"))
}

fn fixture(path: &str) -> String {
    format!("{}/tests/fixtures/{path}", env!("CARGO_MANIFEST_DIR"))
}

fn run(args: &[&str]) -> Output {
    bin().args(args).output().unwrap()
}

fn text(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("relate-gram-cli-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[test]
fn help_and_version() {
    let o = run(&["--help"]);
    assert!(o.status.success());
    assert!(text(&o.stdout).contains("relate-gram check FILE..."));
    let o = run(&["--version"]);
    assert!(o.status.success());
    assert!(text(&o.stdout).starts_with("relate-gram "));
}

#[test]
fn check_a_valid_file_succeeds() {
    let o = run(&["check", &fixture("valid/people.gram")]);
    assert!(o.status.success(), "{}", text(&o.stderr));
    assert!(text(&o.stdout).contains("checked 1 file(s): 0 error(s)"));
}

#[test]
fn check_an_invalid_file_shows_the_source_and_a_marker_and_fails() {
    let path = fixture("invalid/rel-no-type.gram");
    let o = run(&["check", &path]);
    assert_eq!(o.status.code(), Some(1));
    let err = text(&o.stderr);
    assert!(
        err.contains("error[rel.no-type]: a relationship needs a type"),
        "{err}"
    );
    assert!(err.contains(&format!("--> {path}:2:4")), "{err}");
    assert!(err.contains("2 | (a)-->(b)"), "{err}");
    assert!(err.contains("  |    ^^^"), "{err}");
    assert!(text(&o.stdout).contains("1 error(s)"));
}

#[test]
fn check_reads_standard_input() {
    let mut child = bin()
        .args(["check", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"(a)--(b)").unwrap();
    let o = child.wait_with_output().unwrap();
    assert_eq!(o.status.code(), Some(1));
    assert!(
        text(&o.stderr).contains("--> <stdin>:1:4"),
        "{}",
        text(&o.stderr)
    );
}

#[test]
fn check_several_files_counts_them_all() {
    let o = run(&[
        "check",
        &fixture("valid/people.gram"),
        &fixture("invalid/rel-undirected.gram"),
        &fixture("invalid/key-empty.gram"),
    ]);
    assert_eq!(o.status.code(), Some(1));
    assert!(
        text(&o.stdout).contains("checked 3 file(s): 2 error(s)"),
        "{}",
        text(&o.stdout)
    );
}

#[test]
fn warnings_are_shown_unless_quiet() {
    let path = fixture("valid/identity.gram");
    let loud = run(&["check", &path]);
    assert!(text(&loud.stderr).contains("warning[id.reference-only]"));
    let quiet = run(&["check", "-q", &path]);
    assert!(!text(&quiet.stderr).contains("warning["));
    assert!(quiet.status.success());
}

#[test]
fn the_tags_option_turns_a_dropped_tag_into_an_error() {
    let path = fixture("valid/values.gram");
    assert!(run(&["check", &path]).status.success());
    let o = run(&["check", "--tags", "error", &path]);
    assert_eq!(o.status.code(), Some(1));
    assert!(text(&o.stderr).contains("error[value.tag]"));
}

#[test]
fn usage_mistakes_exit_with_two() {
    for args in [
        vec!["frobnicate"],
        vec!["check"],
        vec!["check", "--nope", "x.gram"],
        vec!["load", "x.gram"],
        vec!["export"],
        vec!["normalize"],
        vec!["check", "--tags", "maybe", "x.gram"],
        vec!["check", "--max-errors", "0", "x.gram"],
        vec!["check", "--db"],
    ] {
        let o = run(&args);
        assert_eq!(o.status.code(), Some(2), "{args:?}: {}", text(&o.stderr));
        assert!(text(&o.stderr).contains("relate-gram:"), "{args:?}");
    }
    let o = run(&["check", "/no/such/file.gram"]);
    assert_eq!(o.status.code(), Some(2));
    assert!(text(&o.stderr).contains("cannot read"));
}

#[test]
fn normalize_prints_canonical_gram() {
    let o = run(&["normalize", &fixture("reload/v1.gram")]);
    assert!(o.status.success(), "{}", text(&o.stderr));
    assert_eq!(
        text(&o.stdout),
        "(ada:Person {born: 1815, name: \"Ada\", note: \"kept\"})\n\
         (grace:Person {name: \"Grace\"})\n\
         \n\
         (ada)-[k:KNOWS {since: 1840}]->(grace)\n"
    );
}

#[test]
fn normalize_rejects_an_invalid_file_and_writes_nothing() {
    let o = run(&["normalize", &fixture("invalid/rel-no-type.gram")]);
    assert_eq!(o.status.code(), Some(1));
    assert!(o.stdout.is_empty());
}

#[test]
fn normalize_can_write_to_a_file() {
    let dir = scratch("out");
    std::fs::create_dir_all(&dir).unwrap();
    let out = dir.join("out.gram");
    let o = run(&[
        "normalize",
        &fixture("reload/v1.gram"),
        "-o",
        out.to_str().unwrap(),
    ]);
    assert!(o.status.success());
    assert!(o.stdout.is_empty());
    assert!(std::fs::read_to_string(&out)
        .unwrap()
        .starts_with("(ada:Person"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn load_then_export_through_a_database_on_disk() {
    let dir = scratch("db");
    let db = dir.to_str().unwrap();
    let o = run(&["load", &fixture("reload/v1.gram"), "--db", db]);
    assert!(o.status.success(), "{}", text(&o.stderr));
    assert!(
        text(&o.stdout).contains("2 node(s) and 1 relationship(s) created"),
        "{}",
        text(&o.stdout)
    );

    // a new process opens the database again and sees the graph
    let o = run(&["export", "--db", db]);
    assert!(o.status.success(), "{}", text(&o.stderr));
    assert_eq!(
        text(&o.stdout),
        "(ada:Person {born: 1815, name: \"Ada\", note: \"kept\"})\n\
         (grace:Person {name: \"Grace\"})\n\
         \n\
         (ada)-[k:KNOWS {since: 1840}]->(grace)\n"
    );

    // loading the edited file matches the stored elements instead of duplicating them
    let o = run(&["load", &fixture("reload/v2.gram"), "--db", db]);
    assert!(o.status.success(), "{}", text(&o.stderr));
    assert!(
        text(&o.stdout).contains("1 node(s) and 1 relationship(s) created, 2 and 1 updated"),
        "{}",
        text(&o.stdout)
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_failed_load_says_the_database_is_unchanged() {
    let dir = scratch("fail");
    let db = dir.to_str().unwrap();
    let o = run(&["load", &fixture("invalid/rel-no-type.gram"), "--db", db]);
    assert_eq!(o.status.code(), Some(1));
    assert!(text(&o.stderr).contains("the database is unchanged"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn export_options_choose_identifiers() {
    let dir = scratch("ids");
    let db = dir.to_str().unwrap();
    assert!(run(&["load", &fixture("reload/v1.gram"), "--db", db])
        .status
        .success());
    let o = run(&["export", "--db", db, "--identity", "generate"]);
    assert!(
        text(&o.stdout).starts_with("(n0:Person"),
        "{}",
        text(&o.stdout)
    );
    let o = run(&["export", "--db", db, "--identity", "property"]);
    assert_eq!(o.status.code(), Some(1));
    assert!(text(&o.stderr).contains("export.config"));
    let _ = std::fs::remove_dir_all(&dir);
}

// --- fix ---------------------------------------------------------------

fn gram_in(dir: &std::path::Path, name: &str, text: &str) -> PathBuf {
    std::fs::create_dir_all(dir).unwrap();
    let p = dir.join(name);
    std::fs::write(&p, text).unwrap();
    p
}

const UNTYPED: &str = "// two people\n(a:N)-->(b:N)\n(a)-->(c:N)\n";

#[test]
fn fix_writes_a_new_file_and_leaves_the_original_alone() {
    let dir = scratch("fix-new");
    let input = gram_in(&dir, "people.gram", UNTYPED);
    let o = run(&["fix", input.to_str().unwrap(), "--rel-type", "KNOWS"]);
    assert!(o.status.success(), "{}", text(&o.stderr));
    let out = dir.join("people.fixed.gram");
    assert_eq!(
        std::fs::read_to_string(&out).unwrap(),
        "// two people\n(a:N)-[:KNOWS]->(b:N)\n(a)-[:KNOWS]->(c:N)\n"
    );
    assert_eq!(std::fs::read_to_string(&input).unwrap(), UNTYPED);
    let err = text(&o.stderr);
    assert!(err.contains("applied 2 fix(es)"), "{err}");
    assert!(err.contains("2 x rel.no-type"), "{err}");
    assert!(err.contains("people.fixed.gram"), "{err}");
    // the fixed file passes check
    assert!(run(&["check", out.to_str().unwrap()]).status.success());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn fix_never_replaces_an_existing_output() {
    let dir = scratch("fix-exists");
    let input = gram_in(&dir, "people.gram", UNTYPED);
    let existing = gram_in(&dir, "people.fixed.gram", "(keep)\n");
    let o = run(&["fix", input.to_str().unwrap(), "--rel-type", "KNOWS"]);
    assert_eq!(o.status.code(), Some(2));
    assert!(
        text(&o.stderr).contains("already exists"),
        "{}",
        text(&o.stderr)
    );
    assert_eq!(std::fs::read_to_string(&existing).unwrap(), "(keep)\n");
    // --force replaces it
    let o = run(&[
        "fix",
        input.to_str().unwrap(),
        "--rel-type",
        "KNOWS",
        "--force",
    ]);
    assert!(o.status.success(), "{}", text(&o.stderr));
    assert!(std::fs::read_to_string(&existing)
        .unwrap()
        .contains("KNOWS"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn fix_names_the_output_for_a_file_that_does_not_end_in_gram() {
    let dir = scratch("fix-name");
    let input = gram_in(&dir, "people.txt", UNTYPED);
    let o = run(&["fix", input.to_str().unwrap(), "--rel-type", "K"]);
    assert!(o.status.success(), "{}", text(&o.stderr));
    assert!(dir.join("people.txt.fixed").exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn fix_can_write_to_a_named_file_or_standard_output() {
    let dir = scratch("fix-o");
    let input = gram_in(&dir, "people.gram", UNTYPED);
    let named = dir.join("named.gram");
    let o = run(&[
        "fix",
        input.to_str().unwrap(),
        "--rel-type",
        "K",
        "-o",
        named.to_str().unwrap(),
    ]);
    assert!(o.status.success());
    assert!(std::fs::read_to_string(&named).unwrap().contains("[:K]"));
    assert!(!dir.join("people.fixed.gram").exists());
    let o = run(&["fix", input.to_str().unwrap(), "--rel-type", "K", "-o", "-"]);
    assert!(o.status.success());
    assert!(text(&o.stdout).contains("(a:N)-[:K]->(b:N)"));
    assert!(!dir.join("people.fixed.gram").exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn fix_reads_standard_input_and_writes_standard_output() {
    let mut child = bin()
        .args(["fix", "-", "--rel-type", "K"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"(a:N)-->(b:N)\n")
        .unwrap();
    let o = child.wait_with_output().unwrap();
    assert!(o.status.success(), "{}", text(&o.stderr));
    assert_eq!(text(&o.stdout), "(a:N)-[:K]->(b:N)\n");
}

#[test]
fn fix_in_place_keeps_a_backup() {
    let dir = scratch("fix-inplace");
    let input = gram_in(&dir, "people.gram", UNTYPED);
    let o = run(&[
        "fix",
        input.to_str().unwrap(),
        "--rel-type",
        "K",
        "--in-place",
    ]);
    assert!(o.status.success(), "{}", text(&o.stderr));
    assert!(std::fs::read_to_string(&input).unwrap().contains("[:K]"));
    assert_eq!(
        std::fs::read_to_string(dir.join("people.gram.orig")).unwrap(),
        UNTYPED
    );
    assert!(!dir.join("people.fixed.gram").exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn fix_in_place_does_not_overwrite_a_backup_and_can_skip_it() {
    let dir = scratch("fix-backup");
    let input = gram_in(&dir, "people.gram", UNTYPED);
    gram_in(&dir, "people.gram.orig", "(older)\n");
    let o = run(&[
        "fix",
        input.to_str().unwrap(),
        "--rel-type",
        "K",
        "--in-place",
    ]);
    assert_eq!(o.status.code(), Some(2));
    assert_eq!(
        std::fs::read_to_string(&input).unwrap(),
        UNTYPED,
        "the input is untouched"
    );
    let o = run(&[
        "fix",
        input.to_str().unwrap(),
        "--rel-type",
        "K",
        "--in-place",
        "--no-backup",
    ]);
    assert!(o.status.success(), "{}", text(&o.stderr));
    assert_eq!(
        std::fs::read_to_string(dir.join("people.gram.orig")).unwrap(),
        "(older)\n"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn fix_with_nothing_to_fix_writes_nothing() {
    let dir = scratch("fix-none");
    let input = gram_in(&dir, "ok.gram", "(a:N)-[:T]->(b:N)\n");
    let o = run(&["fix", input.to_str().unwrap()]);
    assert!(o.status.success());
    assert!(
        text(&o.stderr).contains("nothing to fix"),
        "{}",
        text(&o.stderr)
    );
    assert!(!dir.join("ok.fixed.gram").exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn fix_without_the_needed_option_says_which_one_and_writes_nothing() {
    let dir = scratch("fix-hint");
    let input = gram_in(&dir, "people.gram", UNTYPED);
    let o = run(&["fix", input.to_str().unwrap()]);
    assert_eq!(o.status.code(), Some(1));
    let err = text(&o.stderr);
    assert!(err.contains("error[rel.no-type]"), "{err}");
    assert!(err.contains("--rel-type"), "{err}");
    assert!(!dir.join("people.fixed.gram").exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn fix_writes_what_it_could_and_exits_one_when_errors_remain() {
    let dir = scratch("fix-partial");
    let input = gram_in(&dir, "mixed.gram", "(a:N)-->(b:N)\n(c {r: 1..2})\n");
    let o = run(&["fix", input.to_str().unwrap(), "--rel-type", "K"]);
    assert_eq!(o.status.code(), Some(1));
    let out = dir.join("mixed.fixed.gram");
    assert_eq!(
        std::fs::read_to_string(&out).unwrap(),
        "(a:N)-[:K]->(b:N)\n(c {r: 1..2})\n"
    );
    let err = text(&o.stderr);
    assert!(err.contains("error[value.range]"), "{err}");
    assert!(
        err.contains("mixed.fixed.gram:2:"),
        "remaining errors name the fixed file: {err}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn fix_usage_mistakes() {
    for args in [
        vec!["fix"],
        vec!["fix", "a.gram", "b.gram"],
        vec!["fix", "a.gram", "--direction", "up"],
        vec!["fix", "a.gram", "--octal", "hex"],
        vec!["fix", "a.gram", "--rel-type"],
        vec!["fix", "a.gram", "--in-place", "-o", "x.gram"],
        vec!["fix", "-", "--in-place"],
    ] {
        let o = run(&args);
        assert_eq!(o.status.code(), Some(2), "{args:?}: {}", text(&o.stderr));
    }
}

#[test]
fn fix_tags_error_removes_tags() {
    let dir = scratch("fix-tags");
    let input = gram_in(&dir, "t.gram", "(a {s: md`x`})\n");
    let o = run(&["fix", input.to_str().unwrap(), "--tags", "error"]);
    assert!(o.status.success(), "{}", text(&o.stderr));
    assert_eq!(
        std::fs::read_to_string(dir.join("t.fixed.gram")).unwrap(),
        "(a {s: `x`})\n"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
