//! Load gram files into a Relate database and export a Relate database as gram.
//!
//! Behavior is decided by the ADRs under `design/adr/`; see
//! `specs/001-phase1-load-export/` for the specification and contracts.

#![forbid(unsafe_code)]

pub mod diagnostics;
pub mod export;
pub(crate) mod load;
pub mod names;
pub mod plan;
pub mod render;
pub mod settings;
pub mod syntax;
pub mod values;

use diagnostics::{code, Diagnostic};
pub use export::{export, ExportFailure};
use relate::Connection;
use settings::LoadSettings;

/// What a successful load did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LoadCounts {
    pub nodes_created: usize,
    pub nodes_updated: usize,
    pub relationships_created: usize,
    pub relationships_updated: usize,
    /// Elements created without an identifier (ADR-004).
    pub elements_without_identifier: usize,
    /// Nodes created only because they were referenced (ADR-001 section 2.3).
    pub reference_only_nodes: usize,
    pub tags_dropped: usize,
}

/// A successful load: warnings and counts.
#[derive(Clone, Debug)]
pub struct LoadReport {
    pub diagnostics: Vec<Diagnostic>,
    pub counts: LoadCounts,
}

/// A failed load. The database is unchanged.
#[derive(Clone, Debug, thiserror::Error)]
#[error("the load failed with {total} diagnostic(s)")]
pub struct LoadFailure {
    /// Up to `max_diagnostics`, in source order.
    pub diagnostics: Vec<Diagnostic>,
    /// How many errors were found before the cap.
    pub total: usize,
}

impl LoadFailure {
    /// Errors first (capped, with `total` their number), then warnings (capped),
    /// each in source order.
    fn new(diagnostics: Vec<Diagnostic>, max: usize) -> Self {
        let (mut errors, mut warnings): (Vec<_>, Vec<_>) =
            diagnostics.into_iter().partition(Diagnostic::is_error);
        let total = errors.len();
        errors.truncate(max);
        warnings.truncate(max);
        errors.extend(warnings);
        LoadFailure {
            diagnostics: errors,
            total,
        }
    }
}

/// The result of [`check`]: every error and warning, in source order.
#[derive(Clone, Debug)]
pub struct CheckReport {
    pub diagnostics: Vec<Diagnostic>,
}

impl CheckReport {
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(Diagnostic::is_error)
    }
}

/// Parse and plan `source` as a load would, without a database. It finds every
/// error a load finds except those that depend on what a database holds (`match.*`).
pub fn check(source: &[u8], settings: &LoadSettings) -> CheckReport {
    let syn = match syntax::parse(source, settings.max_diagnostics) {
        Ok(s) => s,
        Err(f) => {
            return CheckReport {
                diagnostics: f.diagnostics,
            }
        }
    };
    let plan = plan::build(&syn, settings);
    let mut diagnostics = plan.diagnostics.clone();
    diagnostics.extend(
        plan.nodes
            .iter()
            .filter_map(plan::PlannedNode::reference_only_warning),
    );
    diagnostics.sort_by_key(|d| d.span.map(|s| s.start).unwrap_or(usize::MAX));
    CheckReport { diagnostics }
}

/// Load gram text into the database behind `conn` (contracts/library-api.md).
pub fn load(
    conn: &mut Connection,
    source: &[u8],
    settings: &LoadSettings,
) -> Result<LoadReport, LoadFailure> {
    if conn.in_transaction() {
        return Err(LoadFailure::new(
            vec![Diagnostic::error_without_span(
                code::LOAD_IN_TRANSACTION,
                "the connection is already in a transaction; commit or roll it back first",
            )],
            settings.max_diagnostics,
        ));
    }
    let syn = syntax::parse(source, settings.max_diagnostics).map_err(|f| LoadFailure {
        diagnostics: f.diagnostics,
        total: f.total,
    })?;
    let plan = plan::build(&syn, settings);
    if plan.has_errors() {
        return Err(LoadFailure::new(plan.diagnostics, settings.max_diagnostics));
    }
    let written = load::write(conn, &plan, settings).map_err(|errors| {
        let mut all = plan.diagnostics.clone();
        all.extend(errors);
        LoadFailure::new(all, settings.max_diagnostics)
    })?;

    let mut diagnostics = plan.diagnostics.clone();
    diagnostics.extend(written.warnings.iter().cloned());
    let anonymous = plan
        .nodes
        .iter()
        .filter(|n| plan::node_identity(n, settings).is_none())
        .count()
        + plan
            .rels
            .iter()
            .filter(|r| plan::rel_identity(r, settings).is_none())
            .count();
    if anonymous > 0 {
        diagnostics.push(Diagnostic::warning_without_span(
            code::LOAD_ANONYMOUS,
            format!("{anonymous} element(s) were loaded without an identifier"),
        ));
    }
    diagnostics.sort_by_key(|d| d.span.map(|s| s.start).unwrap_or(usize::MAX));
    let counts = LoadCounts {
        nodes_created: written.nodes_created,
        nodes_updated: written.nodes_updated,
        relationships_created: written.relationships_created,
        relationships_updated: written.relationships_updated,
        elements_without_identifier: anonymous,
        reference_only_nodes: written.warnings.len(),
        tags_dropped: diagnostics
            .iter()
            .filter(|d| d.code == code::VALUE_TAG_DROPPED)
            .count(),
    };
    Ok(LoadReport {
        diagnostics,
        counts,
    })
}

/// Test hooks. Not part of the public API.
#[doc(hidden)]
pub mod __test_support {
    pub use crate::syntax::{parse, Syntax, SyntaxFailure};
}
