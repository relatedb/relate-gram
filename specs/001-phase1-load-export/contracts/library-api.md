# Contract: Library API

The public interface of the `relate-gram` crate in this phase. Rust types are given to fix the contract; field and variant details follow [data-model.md](../data-model.md). Behavior is defined by the ADRs cited in the spec.

## Load

```rust
pub fn load(
    conn: &mut relate::Connection,
    source: &[u8],
    settings: &LoadSettings,
) -> Result<LoadReport, LoadFailure>;
```

- `source` is the raw bytes of a gram file (ADR-007 §2.2: UTF-8, optional byte order mark).
- Precondition: `conn` is not in a transaction. Otherwise the result is `LoadFailure` with the code `load.in-transaction`, and the database is untouched.
- On `Ok`, the graph is written and committed, and the report lists warnings and counts.
- On `Err`, the database is unchanged (FR-001) and `LoadFailure` holds every diagnostic found, up to `max_diagnostics`, with the total, in source order. Errors from Relate itself are returned as a diagnostic with code `load.database-error` that has no source span.
- A load that succeeds twice on the same file leaves the database as it was after the first (ADR-001 §2.7; SC-003).

## Export

```rust
pub fn export(
    conn: &mut relate::Connection,
    settings: &ExportSettings,
) -> Result<String, ExportFailure>;
```

- Precondition: `conn` is not in a transaction.
- On `Ok`, the string is the whole gram text, in the layout of ADR-002 §2.5; two calls on an unchanged database return equal strings (SC-004).
- On `Err`, no text is produced, and `ExportFailure` lists every offending element or setting (ADR-002 §2.7; ADR-006 §2.6).
- The database is read in one transaction and never modified.

## Types

```rust
pub struct LoadSettings { pub node_id_property: Option<String>, pub relationship_id_property: Option<String>,
                          pub tags: TagPolicy, pub max_diagnostics: usize }
pub enum TagPolicy { Warn, Error }

pub struct ExportSettings { pub identity: IdentityMode, pub node_id_property: Option<String>,
                            pub relationship_id_property: Option<String> }
pub enum IdentityMode { Auto, Generate, Property }

pub struct LoadReport { pub diagnostics: Vec<Diagnostic>, pub counts: LoadCounts }
pub struct LoadFailure { pub diagnostics: Vec<Diagnostic>, pub total: usize }
pub struct ExportFailure { pub diagnostics: Vec<Diagnostic>, pub total: usize }

pub struct Diagnostic { pub code: &'static str, pub severity: Severity, pub span: Option<Span>,
                        pub message: String, pub related: Vec<(String, Span)> }
```

`Default` implementations give the defaults the ADRs state (`TagPolicy::Warn`, `IdentityMode::Auto`, no identity properties, 100 diagnostics).

## Compatibility

Diagnostic codes are stable across releases within a major version; messages are not. The library does not read the environment or print anything: warnings and errors are returned, and a caller decides how to show them.
