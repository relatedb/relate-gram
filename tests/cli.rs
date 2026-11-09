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
