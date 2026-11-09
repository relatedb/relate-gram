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
pub mod settings;
pub mod syntax;
pub mod values;

use diagnostics::{code, Diagnostic};
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
    /// How many were found before the cap.
    pub total: usize,
}

impl LoadFailure {
    fn new(mut diagnostics: Vec<Diagnostic>, max: usize) -> Self {
        let total = diagnostics.len();
        diagnostics.truncate(max);
        LoadFailure { diagnostics, total }
    }
}

/// Load gram text into the database behind `conn` (contracts/library-api.md).
pub fn load(
    conn: &mut Connection,
    source: &[u8],
    settings: &LoadSettings,
) -> Result<LoadReport, LoadFailure> {
    let syn = syntax::parse(source, settings.max_diagnostics).map_err(|f| LoadFailure {
        diagnostics: f.diagnostics,
        total: f.total,
    })?;
    let plan = plan::build(&syn, settings);
    if plan.has_errors() {
        return Err(LoadFailure::new(plan.diagnostics, settings.max_diagnostics));
    }
    let written = load::write(conn, &plan)
        .map_err(|d| LoadFailure::new(vec![d], settings.max_diagnostics))?;

    let mut diagnostics = plan.diagnostics.clone();
    let anonymous = plan.nodes.iter().filter(|n| n.key.is_none()).count()
        + plan.rels.iter().filter(|r| r.key.is_none()).count();
    if anonymous > 0 {
        diagnostics.push(Diagnostic::warning_without_span(
            code::LOAD_ANONYMOUS,
            format!("{anonymous} element(s) were loaded without an identifier"),
        ));
    }
    let counts = LoadCounts {
        nodes_created: written.nodes_created,
        relationships_created: written.relationships_created,
        elements_without_identifier: anonymous,
        reference_only_nodes: plan
            .nodes
            .iter()
            .filter(|n| n.key.is_some() && !n.defined)
            .count(),
        tags_dropped: diagnostics
            .iter()
            .filter(|d| d.code == code::VALUE_TAG_DROPPED)
            .count(),
        ..LoadCounts::default()
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
