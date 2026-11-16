//! Statuses, known failures and the results file (ADR-015 section 2.6).
use gram_codec::parse_gram_with_header;
use pattern_core::Value;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Passed,
    Failed,
    /// A known failure that failed, as it must.
    Skipped,
    NotApplicable,
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Passed => "passed",
            Status::Failed => "failed",
            Status::Skipped => "skipped",
            Status::NotApplicable => "not-applicable",
        }
    }
}

/// A known failure: its reason and issue.
pub type Known = (String, String);

/// `(:KnownFailure {case, reason, issue})` nodes, by case (`kind/id`).
pub fn known_failures(text: &str) -> Result<BTreeMap<String, Known>, String> {
    let (_, patterns) = parse_gram_with_header(text).map_err(|e| e.to_string())?;
    let mut out = BTreeMap::new();
    for p in &patterns {
        let get = |k: &str| match p.value.properties.get(k) {
            Some(Value::VString(s)) | Some(Value::VSymbol(s)) => Ok(s.clone()),
            _ => Err(format!("a KnownFailure needs a `{k}`")),
        };
        out.insert(get("case")?, (get("reason")?, get("issue")?));
    }
    Ok(out)
}

/// The status of one case, and why it failed if it did. `outcome` is `None` when the case was not
/// run.
pub fn decide(
    applicable: bool,
    outcome: Option<Result<(), String>>,
    known: Option<&Known>,
) -> (Status, Option<String>) {
    if !applicable {
        return (Status::NotApplicable, None);
    }
    match (outcome.unwrap_or(Ok(())), known) {
        (Ok(()), None) => (Status::Passed, None),
        (Err(e), None) => (Status::Failed, Some(e)),
        (Err(_), Some(_)) => (Status::Skipped, None),
        (Ok(()), Some((reason, _))) => (
            Status::Failed,
            Some(format!(
                "this known failure passed (reason given: {reason}); remove it from known-failures.gram"
            )),
        ),
    }
}

/// The text of a results file: one `Result` per case, in the shape of gram-value's results.
pub fn results_text(rows: &[(String, Status)], version: &str) -> String {
    let mut out = String::from("{kind: \"conformance-results\"}\n");
    for (case, status) in rows {
        out.push_str(&format!(
            "(:Result {{case: \"{case}\", status: \"{}\", version: \"{version}\"}})\n",
            status.as_str()
        ));
    }
    out
}
