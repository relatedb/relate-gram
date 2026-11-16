//! The `export` kind (ADR-015 section 2.2): a database made by `start.cypher`, exported with the
//! case's settings. The text is compared byte for byte; a failure is compared by code, as a
//! multiset, because the elements it names have internal identifiers (section 2.5).
use super::case::Case;
use super::{db, expect, settings};

/// The first byte at which two texts differ, for a message.
pub fn first_difference(expected: &[u8], got: &[u8]) -> String {
    let at = expected
        .iter()
        .zip(got)
        .position(|(a, b)| a != b)
        .unwrap_or(expected.len().min(got.len()));
    let show = |t: &[u8]| {
        String::from_utf8_lossy(&t[at.saturating_sub(10)..(at + 20).min(t.len())]).into_owned()
    };
    format!(
        "the text differs at byte {at} (expected {} bytes, got {}): expected `{}`, got `{}`",
        expected.len(),
        got.len(),
        show(expected),
        show(got)
    )
}

/// Run an `export` case on its starting statements and its expected text, if any.
pub fn run_export_text(case: &Case, start: &str, exported: Option<&[u8]>) -> Result<(), String> {
    let export_settings = settings::export_settings(&case.settings)?;
    let (_db, mut conn) = db::fresh(start)?;
    if case.transaction {
        conn.begin().map_err(|e| e.to_string())?;
    }
    match relate_gram::export(&mut conn, &export_settings) {
        Ok(text) => {
            if !case.errors().is_empty() {
                return Err(format!(
                    "expected the export to fail with {}, but it wrote {} bytes",
                    case.errors()
                        .iter()
                        .map(|e| e.code.clone())
                        .collect::<Vec<_>>()
                        .join(", "),
                    text.len()
                ));
            }
            match exported {
                Some(want) if want == text.as_bytes() => Ok(()),
                Some(want) => Err(first_difference(want, text.as_bytes())),
                None => Err("the export succeeded but the case has no exported.txt".into()),
            }
        }
        Err(f) => {
            let (errors, _) = expect::split(&f.diagnostics);
            // an export failure is compared by code only
            let codes_only = |ds: Vec<super::case::Diag>| {
                ds.into_iter()
                    .map(|d| super::case::Diag {
                        code: d.code,
                        position: None,
                    })
                    .collect::<Vec<_>>()
            };
            expect::compare_diagnostics("Error", &codes_only(case.errors()), &codes_only(errors))
        }
    }
}

/// Run an `export` case from its directory.
pub fn run_export(case: &Case) -> Result<(), String> {
    let start = std::fs::read_to_string(case.dir.join("start.cypher"))
        .map_err(|e| format!("start.cypher: {e}"))?;
    let exported = std::fs::read(case.dir.join("exported.txt")).ok();
    run_export_text(case, &start, exported.as_deref())
}
