//! A case: `case.gram` and the files beside it (ADR-015 section 2.2).
use gram_codec::parse_gram_with_header;
use pattern_core::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Kind {
    Reject,
    Load,
    Export,
    Fix,
}

impl Kind {
    pub const ALL: [Kind; 4] = [Kind::Reject, Kind::Load, Kind::Export, Kind::Fix];

    pub fn dir(self) -> &'static str {
        match self {
            Kind::Reject => "reject",
            Kind::Load => "load",
            Kind::Export => "export",
            Kind::Fix => "fix",
        }
    }

    /// The files a case of this kind may hold, and which of them it must hold.
    fn files(self) -> (&'static [&'static str], &'static [&'static str]) {
        match self {
            Kind::Reject => (
                &["case.gram", "input.txt", "input.bin", "start.cypher"],
                &["case.gram"],
            ),
            Kind::Load => (
                &["case.gram", "input.txt", "expected.gram", "start.cypher", "exported.txt"],
                &["case.gram", "input.txt", "expected.gram"],
            ),
            Kind::Export => (&["case.gram", "start.cypher", "exported.txt"], &["case.gram", "start.cypher"]),
            Kind::Fix => (&["case.gram", "input.txt", "fixed.txt"], &["case.gram", "input.txt", "fixed.txt"]),
        }
    }
}

/// A setting value from the `settings` map of `case.gram`.
#[derive(Clone, Debug, PartialEq)]
pub enum Setting {
    Str(String),
    Int(i64),
    Bool(bool),
}

/// An expected diagnostic: a code, and a position when the diagnostic has one.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Diag {
    pub code: String,
    /// (line, column, byte offset).
    pub position: Option<(usize, usize, usize)>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Expectation {
    Error(Diag),
    Warning(Diag),
    Remaining(Diag),
    Counts(BTreeMap<String, i64>),
    Applied { code: String, count: i64 },
}

#[derive(Clone, Debug)]
pub struct Case {
    pub kind: Kind,
    pub id: String,
    pub dir: PathBuf,
    pub requires: Vec<String>,
    pub settings: BTreeMap<String, Setting>,
    pub transaction: bool,
    pub expectations: Vec<Expectation>,
}

impl Case {
    pub fn errors(&self) -> Vec<Diag> {
        self.expectations
            .iter()
            .filter_map(|e| if let Expectation::Error(d) = e { Some(d.clone()) } else { None })
            .collect()
    }

    pub fn warnings(&self) -> Vec<Diag> {
        self.expectations
            .iter()
            .filter_map(|e| if let Expectation::Warning(d) = e { Some(d.clone()) } else { None })
            .collect()
    }

    pub fn remaining(&self) -> Vec<Diag> {
        self.expectations
            .iter()
            .filter_map(|e| if let Expectation::Remaining(d) = e { Some(d.clone()) } else { None })
            .collect()
    }

    pub fn counts(&self) -> Option<&BTreeMap<String, i64>> {
        self.expectations.iter().find_map(|e| if let Expectation::Counts(m) = e { Some(m) } else { None })
    }

    pub fn applied(&self) -> Vec<(String, i64)> {
        self.expectations
            .iter()
            .filter_map(|e| if let Expectation::Applied { code, count } = e { Some((code.clone(), *count)) } else { None })
            .collect()
    }
}

const COUNTS_KEYS: [&str; 8] = [
    "nodesCreated",
    "nodesUpdated",
    "relationshipsCreated",
    "relationshipsUpdated",
    "elementsWithoutIdentifier",
    "referenceOnlyNodes",
    "tagsDropped",
    "total",
];

fn text_of(v: &Value) -> Option<String> {
    match v {
        Value::VString(s) | Value::VSymbol(s) => Some(s.clone()),
        _ => None,
    }
}

fn setting_of(v: &Value) -> Option<Setting> {
    match v {
        Value::VString(s) | Value::VSymbol(s) => Some(Setting::Str(s.clone())),
        Value::VInteger(i) => Some(Setting::Int(*i)),
        Value::VBoolean(b) => Some(Setting::Bool(*b)),
        _ => None,
    }
}

fn int_prop(props: &std::collections::HashMap<String, Value>, key: &str) -> Result<Option<usize>, String> {
    match props.get(key) {
        None => Ok(None),
        Some(Value::VInteger(i)) if *i >= 0 => Ok(Some(*i as usize)),
        Some(other) => Err(format!("`{key}` must be a non-negative integer, found {other:?}")),
    }
}

fn diag_of(label: &str, props: &std::collections::HashMap<String, Value>) -> Result<Diag, String> {
    let code = props
        .get("code")
        .and_then(text_of)
        .ok_or_else(|| format!("{label} needs a `code`"))?;
    let (line, column, offset) = (int_prop(props, "line")?, int_prop(props, "column")?, int_prop(props, "offset")?);
    let position = match (line, column, offset) {
        (Some(l), Some(c), Some(o)) => Some((l, c, o)),
        (None, None, None) => None,
        _ => return Err(format!("{label} `{code}` needs all of `line`, `column` and `offset`, or none")),
    };
    for k in props.keys() {
        if !["code", "line", "column", "offset"].contains(&k.as_str()) {
            return Err(format!("{label} has an unknown key `{k}`"));
        }
    }
    Ok(Diag { code, position })
}

/// Read the text of a `case.gram`. The error names the case.
pub fn parse_case(kind: Kind, id: &str, text: &str) -> Result<Case, String> {
    let fail = |m: String| format!("case {}/{id}: {m}", kind.dir());
    let (header, patterns) = parse_gram_with_header(text).map_err(|e| fail(format!("case.gram does not parse: {e}")))?;
    let header = header.ok_or_else(|| fail("case.gram has no header record with `requires`".into()))?;
    let mut requires = None;
    let mut settings = BTreeMap::new();
    let mut transaction = false;
    for (k, v) in &header {
        match (k.as_str(), v) {
            ("requires", Value::VArray(items)) => {
                requires = Some(
                    items
                        .iter()
                        .map(|i| text_of(i).ok_or_else(|| fail("`requires` holds only names".into())))
                        .collect::<Result<Vec<_>, _>>()?,
                );
            }
            ("settings", Value::VMap(m)) => {
                for (sk, sv) in m {
                    let s = setting_of(sv).ok_or_else(|| fail(format!("setting `{sk}` must be a string, integer or boolean")))?;
                    settings.insert(sk.clone(), s);
                }
            }
            ("transaction", Value::VBoolean(b)) => transaction = *b,
            (other, _) => return Err(fail(format!("unknown or ill-typed header key `{other}`"))),
        }
    }
    let requires = requires.ok_or_else(|| fail("case.gram has no `requires`".into()))?;
    let mut expectations = Vec::new();
    for p in &patterns {
        let s = &p.value;
        let mut labels: Vec<&String> = s.labels.iter().collect();
        labels.sort();
        let label = match labels.as_slice() {
            [l] => l.as_str(),
            _ => return Err(fail(format!("an expectation node has exactly one label, found {labels:?}"))),
        };
        let props = &s.properties;
        let e = match label {
            "Error" => Expectation::Error(diag_of("Error", props).map_err(fail)?),
            "Warning" => Expectation::Warning(diag_of("Warning", props).map_err(fail)?),
            "Remaining" => Expectation::Remaining(diag_of("Remaining", props).map_err(fail)?),
            "Counts" => {
                let mut m = BTreeMap::new();
                for (k, v) in props {
                    if !COUNTS_KEYS.contains(&k.as_str()) {
                        return Err(fail(format!("Counts has an unknown key `{k}`")));
                    }
                    match v {
                        Value::VInteger(i) => {
                            m.insert(k.clone(), *i);
                        }
                        other => return Err(fail(format!("Counts `{k}` must be an integer, found {other:?}"))),
                    }
                }
                Expectation::Counts(m)
            }
            "Applied" => {
                let code = props.get("code").and_then(text_of).ok_or_else(|| fail("Applied needs a `code`".into()))?;
                let count = match props.get("count") {
                    Some(Value::VInteger(i)) => *i,
                    _ => return Err(fail("Applied needs an integer `count`".into())),
                };
                Expectation::Applied { code, count }
            }
            other => return Err(fail(format!("unknown label `{other}`"))),
        };
        expectations.push(e);
    }
    Ok(Case {
        kind,
        id: id.to_string(),
        dir: PathBuf::new(),
        requires,
        settings,
        transaction,
        expectations,
    })
}

/// Read a case directory: check its files against its kind, then read `case.gram`.
pub fn read_case(dir: &Path, kind: Kind, id: &str) -> Result<Case, String> {
    let (allowed, required) = kind.files();
    let mut present = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(|e| format!("case {}/{id}: {e}", kind.dir()))? {
        present.push(entry.map_err(|e| e.to_string())?.file_name().to_string_lossy().to_string());
    }
    for f in &present {
        if !allowed.contains(&f.as_str()) {
            return Err(format!("case {}/{id}: the file `{f}` is not used by a `{}` case", kind.dir(), kind.dir()));
        }
    }
    for r in required {
        let satisfied = if *r == "input.txt" && kind == Kind::Reject {
            true
        } else {
            present.iter().any(|f| f == r)
        };
        if !satisfied {
            return Err(format!("case {}/{id}: the file `{r}` is missing", kind.dir()));
        }
    }
    if kind == Kind::Reject && !present.iter().any(|f| f == "input.txt" || f == "input.bin") {
        return Err(format!("case reject/{id}: the text under test (`input.txt` or `input.bin`) is missing"));
    }
    let text = std::fs::read_to_string(dir.join("case.gram")).map_err(|e| format!("case {}/{id}: {e}", kind.dir()))?;
    let mut case = parse_case(kind, id, &text)?;
    case.dir = dir.to_path_buf();
    Ok(case)
}
