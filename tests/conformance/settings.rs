//! `settings` of a case to the library's settings (ADR-015 section 2.2): lowerCamelCase keys, an
//! absent key means the library's default, an unknown or ill-typed key fails.
use super::case::Setting;
use relate_gram::fix::{Direction, FixOptions, OctalAs};
use relate_gram::settings::{ExportSettings, IdentityMode, LoadSettings, TagPolicy};
use std::collections::BTreeMap;

type Settings = BTreeMap<String, Setting>;

fn string(s: &Settings, key: &str) -> Result<Option<String>, String> {
    match s.get(key) {
        None => Ok(None),
        Some(Setting::Str(v)) => Ok(Some(v.clone())),
        Some(other) => Err(format!("setting `{key}` must be a string, found {other:?}")),
    }
}

fn check_keys(s: &Settings, allowed: &[&str], operation: &str) -> Result<(), String> {
    for k in s.keys() {
        if !allowed.contains(&k.as_str()) {
            return Err(format!("`{k}` is not a setting of {operation}"));
        }
    }
    Ok(())
}

pub fn load_settings(s: &Settings) -> Result<LoadSettings, String> {
    check_keys(
        s,
        &[
            "nodeIdProperty",
            "relationshipIdProperty",
            "tags",
            "maxDiagnostics",
            "fileName",
        ],
        "a load",
    )?;
    let mut out = LoadSettings::default();
    out.node_id_property = string(s, "nodeIdProperty")?;
    out.relationship_id_property = string(s, "relationshipIdProperty")?;
    out.file_name = string(s, "fileName")?;
    if let Some(t) = string(s, "tags")? {
        out.tags = match t.as_str() {
            "warn" => TagPolicy::Warn,
            "error" => TagPolicy::Error,
            other => {
                return Err(format!(
                    "setting `tags` is `warn` or `error`, found `{other}`"
                ))
            }
        };
    }
    match s.get("maxDiagnostics") {
        None => {}
        Some(Setting::Int(i)) if *i >= 0 => out.max_diagnostics = *i as usize,
        Some(other) => {
            return Err(format!(
                "setting `maxDiagnostics` must be a non-negative integer, found {other:?}"
            ))
        }
    }
    Ok(out)
}

pub fn export_settings(s: &Settings) -> Result<ExportSettings, String> {
    check_keys(
        s,
        &["identity", "nodeIdProperty", "relationshipIdProperty"],
        "an export",
    )?;
    let mut out = ExportSettings::default();
    out.node_id_property = string(s, "nodeIdProperty")?;
    out.relationship_id_property = string(s, "relationshipIdProperty")?;
    if let Some(i) = string(s, "identity")? {
        out.identity = match i.as_str() {
            "auto" => IdentityMode::Auto,
            "generate" => IdentityMode::Generate,
            "property" => IdentityMode::Property,
            other => {
                return Err(format!(
                    "setting `identity` is `auto`, `generate` or `property`, found `{other}`"
                ))
            }
        };
    }
    Ok(out)
}

pub fn fix_options(s: &Settings) -> Result<FixOptions, String> {
    check_keys(s, &["relType", "direction", "octal", "annotate"], "a fix")?;
    let mut out = FixOptions::default();
    out.rel_type = string(s, "relType")?;
    if let Some(d) = string(s, "direction")? {
        out.direction = Some(match d.as_str() {
            "right" => Direction::Right,
            "left" => Direction::Left,
            other => {
                return Err(format!(
                    "setting `direction` is `right` or `left`, found `{other}`"
                ))
            }
        });
    }
    if let Some(o) = string(s, "octal")? {
        out.octal = Some(match o.as_str() {
            "decimal" => OctalAs::Decimal,
            "octal" => OctalAs::Octal,
            other => {
                return Err(format!(
                    "setting `octal` is `decimal` or `octal`, found `{other}`"
                ))
            }
        });
    }
    match s.get("annotate") {
        None => {}
        Some(Setting::Bool(b)) => out.annotate = *b,
        Some(other) => {
            return Err(format!(
                "setting `annotate` must be a boolean, found {other:?}"
            ))
        }
    }
    Ok(out)
}
