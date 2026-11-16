//! Finding the collection and its cases.
use super::case::Kind;
use std::path::{Path, PathBuf};

/// The `conformance/` directory of this crate.
pub fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("conformance")
}

/// The cases of one kind, by identifier, in order.
pub fn cases(kind: Kind) -> Vec<(String, PathBuf)> {
    let dir = root().join(kind.dir());
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for e in entries.flatten() {
            if e.path().is_dir() {
                out.push((e.file_name().to_string_lossy().to_string(), e.path()));
            }
        }
    }
    out.sort();
    out
}

/// The standard starting database of a `reject` case (ADR-015 section 2.2).
pub fn standard_start() -> String {
    std::fs::read_to_string(root().join("start.cypher")).expect("conformance/start.cypher")
}
