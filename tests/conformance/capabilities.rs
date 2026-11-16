//! Capabilities decide which cases apply (ADR-015 section 2.6). The probes come from
//! `conformance/capabilities.gram` and run at start-up; a declared file may only remove one.
use super::case::{Case, Kind};
use super::db;
use gram_codec::parse_gram_with_header;
use pattern_core::Value;
use std::collections::BTreeSet;

fn text_of(v: &Value) -> Option<String> {
    match v {
        Value::VString(s) | Value::VSymbol(s) => Some(s.clone()),
        _ => None,
    }
}

/// The `Capability` nodes of a file: name and properties.
fn nodes(text: &str) -> Result<Vec<std::collections::HashMap<String, Value>>, String> {
    let (_, patterns) = parse_gram_with_header(text).map_err(|e| e.to_string())?;
    Ok(patterns
        .iter()
        .map(|p| p.value.properties.clone())
        .collect())
}

/// The names this implementation supports, found by running each probe.
pub fn probe(text: &str) -> Result<BTreeSet<String>, String> {
    let mut present = BTreeSet::new();
    for props in nodes(text)? {
        let name = props
            .get("name")
            .and_then(text_of)
            .ok_or("a Capability needs a `name`")?;
        let how = props
            .get("how")
            .and_then(text_of)
            .ok_or_else(|| format!("capability `{name}` needs `how`"))?;
        let ok = match how.as_str() {
            "cypher" => {
                let q = props
                    .get("probe")
                    .and_then(text_of)
                    .ok_or_else(|| format!("capability `{name}` needs a `probe`"))?;
                let (_db, mut conn) = db::fresh("")?;
                conn.query(&q).is_ok()
            }
            "transaction" => {
                let (_db, mut conn) = db::fresh("")?;
                conn.begin().is_ok() && conn.rollback().is_ok()
            }
            // a property of the runner's gram reader, and the operations this runner offers
            "reader" | "declared" => true,
            other => return Err(format!("capability `{name}`: unknown `how` `{other}`")),
        };
        if ok {
            present.insert(name);
        }
    }
    Ok(present)
}

/// Apply a declared file: `(:Capability {name, supported: false, reason})` removes a capability.
/// Claiming one the probes did not find, or removing without a reason, is refused.
pub fn apply_declared(
    present: &BTreeSet<String>,
    declared: &str,
) -> Result<BTreeSet<String>, String> {
    let mut out = present.clone();
    for props in nodes(declared)? {
        let name = props
            .get("name")
            .and_then(text_of)
            .ok_or("a Capability needs a `name`")?;
        match props.get("supported") {
            Some(Value::VBoolean(false)) => {
                if props.get("reason").and_then(text_of).is_none() {
                    return Err(format!("removing `{name}` needs a `reason`"));
                }
                out.remove(&name);
            }
            Some(Value::VBoolean(true)) if !present.contains(&name) => {
                return Err(format!(
                    "`{name}` cannot be added by a declared file: only a probe can find it"
                ));
            }
            Some(Value::VBoolean(true)) => {}
            _ => {
                return Err(format!(
                    "capability `{name}` needs `supported: true` or `false`"
                ))
            }
        }
    }
    Ok(out)
}

/// A case applies when every requirement is met; `!name` requires the name to be absent.
pub fn applicable(requires: &[String], present: &BTreeSet<String>) -> bool {
    requires.iter().all(|r| match r.strip_prefix('!') {
        Some(name) => !present.contains(name),
        None => present.contains(r),
    })
}

/// The operations a case runs (ADR-015 section 2.6).
pub fn operations(case: &Case) -> Vec<&'static str> {
    match case.kind {
        Kind::Reject => vec!["check", "load"],
        Kind::Load if case.dir.join("exported.txt").exists() => vec!["load", "export"],
        Kind::Load => vec!["load"],
        Kind::Export => vec!["export"],
        Kind::Fix => vec!["fix"],
    }
}
