//! Checks of the collection itself (ADR-015 section 2.8): lint, vocabulary, versions, identifier
//! uniqueness, the attributes that keep byte strings intact, and the coverage of diagnostic codes.
//! Each returns problems as text; an empty list means the collection is sound.
use gram_codec::parse_gram_with_header;
use pattern_core::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

const KINDS: [&str; 4] = ["reject", "load", "export", "fix"];

pub fn has_gram_tool() -> bool {
    Command::new("gram")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

fn text(v: &Value) -> Option<String> {
    match v {
        Value::VString(s) | Value::VSymbol(s) => Some(s.clone()),
        _ => None,
    }
}

/// Every file named `name` under `dir`.
fn files_named(dir: &Path, name: &str, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            files_named(&p, name, out);
        } else if p.file_name().is_some_and(|n| n == name) {
            out.push(p);
        }
    }
}

fn case_files(root: &Path) -> Vec<PathBuf> {
    let mut v = Vec::new();
    files_named(root, "case.gram", &mut v);
    v.sort();
    v
}

fn short(root: &Path, p: &Path) -> String {
    p.strip_prefix(root).unwrap_or(p).display().to_string()
}

/// `gram lint --strict` over the case and expected-graph files and the collection's own gram files.
fn lint(root: &Path, require: bool, problems: &mut Vec<String>) {
    if !has_gram_tool() {
        if require {
            problems.push(
                "lint: the `gram` tool is missing (cargo install gram-data --locked --version 0.7.0)"
                    .into(),
            );
        }
        return;
    }
    let mut files = case_files(root);
    files_named(root, "expected.gram", &mut files);
    for top in [
        "vocabulary.gram",
        "capabilities.gram",
        "known-failures.gram",
    ] {
        if root.join(top).exists() {
            files.push(root.join(top));
        }
    }
    for chunk in files.chunks(100) {
        let out = Command::new("gram")
            .args(["lint", "--strict"])
            .args(chunk)
            .output();
        match out {
            Ok(o) if o.status.success() => {}
            Ok(o) => {
                // name each file that fails, so the message points at it
                for f in chunk {
                    let one = Command::new("gram")
                        .args(["lint", "--strict"])
                        .arg(f)
                        .output();
                    if one.is_ok_and(|r| !r.status.success()) {
                        problems.push(format!("lint: {} does not lint strictly", short(root, f)));
                    }
                }
                if problems.iter().all(|p| !p.starts_with("lint:")) {
                    problems.push(format!(
                        "lint: {}",
                        String::from_utf8_lossy(&o.stdout)
                            .lines()
                            .next()
                            .unwrap_or("failed")
                    ));
                }
            }
            Err(e) => problems.push(format!("lint: {e}")),
        }
    }
}

fn vocabulary(root: &Path, problems: &mut Vec<String>) -> (BTreeSet<String>, BTreeSet<String>) {
    let (mut labels, mut caps) = (BTreeSet::new(), BTreeSet::new());
    let Ok(t) = std::fs::read_to_string(root.join("vocabulary.gram")) else {
        problems.push("vocabulary.gram is missing".into());
        return (labels, caps);
    };
    match parse_gram_with_header(&t) {
        Err(e) => problems.push(format!("vocabulary.gram does not parse: {e}")),
        Ok((_, patterns)) => {
            for p in &patterns {
                let name = p.value.properties.get("name").and_then(text);
                let label = p.value.labels.iter().next().cloned();
                match (label.as_deref(), name) {
                    (Some("Label"), Some(n)) => {
                        labels.insert(n);
                    }
                    (Some("Capability"), Some(n)) => {
                        caps.insert(n);
                    }
                    _ => problems.push(
                        "vocabulary.gram: a node is not a Label or Capability with a name".into(),
                    ),
                }
            }
        }
    }
    (labels, caps)
}

struct Seen {
    codes: BTreeSet<String>,
}

/// Names and codes used by the case files: labels and `requires` against the vocabulary.
fn case_names(
    root: &Path,
    labels: &BTreeSet<String>,
    caps: &BTreeSet<String>,
    problems: &mut Vec<String>,
) -> Seen {
    let mut seen = Seen {
        codes: BTreeSet::new(),
    };
    for f in case_files(root) {
        let Ok(t) = std::fs::read_to_string(&f) else {
            continue;
        };
        let Ok((header, patterns)) = parse_gram_with_header(&t) else {
            problems.push(format!("{} does not parse", short(root, &f)));
            continue;
        };
        for p in &patterns {
            for l in &p.value.labels {
                if !labels.contains(l) {
                    problems.push(format!(
                        "{}: label `{l}` is not in vocabulary.gram",
                        short(root, &f)
                    ));
                }
                if matches!(l.as_str(), "Error" | "Warning" | "Remaining") {
                    if let Some(c) = p.value.properties.get("code").and_then(text) {
                        seen.codes.insert(c);
                    }
                }
            }
        }
        if let Some(Value::VArray(items)) = header.and_then(|h| h.get("requires").cloned()) {
            for i in &items {
                if let Some(n) = text(i) {
                    let bare = n.strip_prefix('!').unwrap_or(&n);
                    if !caps.contains(bare) {
                        problems.push(format!(
                            "{}: capability `{bare}` is not in vocabulary.gram",
                            short(root, &f)
                        ));
                    }
                }
            }
        }
    }
    seen
}

fn versions(root: &Path, problems: &mut Vec<String>) {
    match std::fs::read_to_string(root.join("VERSION")) {
        Err(_) => problems.push("VERSION is missing".into()),
        Ok(v) => {
            let ok = {
                let parts: Vec<&str> = v.trim().split('.').collect();
                parts.len() == 3
                    && parts
                        .iter()
                        .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
            };
            if !ok {
                problems.push(format!("VERSION `{}` is not MAJOR.MINOR.PATCH", v.trim()));
            }
        }
    }
    match std::fs::read_to_string(root.join("GRAMMAR_VERSION")) {
        Err(_) => problems.push("GRAMMAR_VERSION is missing".into()),
        Ok(v) if v.trim().is_empty() => problems.push("GRAMMAR_VERSION is empty".into()),
        Ok(_) => {}
    }
}

fn identifiers(root: &Path, problems: &mut Vec<String>) {
    for kind in KINDS {
        let mut seen: BTreeMap<String, String> = BTreeMap::new();
        let Ok(entries) = std::fs::read_dir(root.join(kind)) else {
            continue;
        };
        let mut names: Vec<String> = entries
            .flatten()
            .filter(|e| e.path().is_dir())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        names.sort();
        for n in names {
            if let Some(other) = seen.insert(n.to_lowercase().replace('_', "-"), n.clone()) {
                problems.push(format!(
                    "{kind}: duplicate case identifier `{n}` (same as `{other}` ignoring case and `_` against `-`)"
                ));
            }
        }
    }
}

fn attributes(root: &Path, problems: &mut Vec<String>) {
    let t = root
        .parent()
        .map(|p| p.join(".gitattributes"))
        .and_then(|p| std::fs::read_to_string(p).ok())
        .unwrap_or_default();
    for rule in ["conformance/**/*.txt -text", "conformance/**/*.bin -text"] {
        if !t.lines().any(|l| l.trim() == rule) {
            problems.push(format!(".gitattributes lacks the rule `{rule}`"));
        }
    }
}

/// Codes the collection need not cover, each with a reason, from `conformance/README.md`: list items
/// of the form ``- `code`: reason`` under the heading "Codes without a case".
fn exceptions(root: &Path) -> BTreeMap<String, String> {
    let t = std::fs::read_to_string(root.join("README.md")).unwrap_or_default();
    let mut out = BTreeMap::new();
    let mut inside = false;
    for line in t.lines() {
        if line.starts_with('#') {
            inside = line
                .trim_start_matches('#')
                .trim()
                .eq_ignore_ascii_case("Codes without a case");
            continue;
        }
        if inside {
            if let Some(rest) = line.strip_prefix("- `") {
                if let Some((code, reason)) = rest.split_once("`:") {
                    out.insert(code.to_string(), reason.trim().to_string());
                }
            }
        }
    }
    out
}

/// Run every check over the collection at `root`. With `require_lint`, a missing `gram` tool is a
/// problem; without it the lint is skipped.
pub fn check_collection(root: &Path, require_lint: bool, codes: &[&str]) -> Vec<String> {
    let mut problems = Vec::new();
    lint(root, require_lint, &mut problems);
    let (labels, caps) = vocabulary(root, &mut problems);
    let seen = case_names(root, &labels, &caps, &mut problems);
    versions(root, &mut problems);
    identifiers(root, &mut problems);
    attributes(root, &mut problems);
    let allowed = exceptions(root);
    for c in codes {
        if !seen.codes.contains(*c) && !allowed.contains_key(*c) {
            problems.push(format!(
                "code `{c}` has no case and no reason in conformance/README.md"
            ));
        }
    }
    problems
}
