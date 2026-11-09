//! Load gram files into a Relate database and export a Relate database as gram.
//!
//! Behavior is decided by the ADRs under `design/adr/`; see
//! `specs/001-phase1-load-export/` for the specification and contracts.

#![forbid(unsafe_code)]

pub mod diagnostics;
pub mod export;
pub mod load;
pub mod names;
pub mod plan;
pub mod settings;
pub mod syntax;
pub mod values;
