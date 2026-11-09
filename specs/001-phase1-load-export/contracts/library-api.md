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
- On `Err`, the database is unchanged (FR-001). `LoadFailure` holds the errors found in source order, up to `max_diagnostics`, with `total` the number of errors found, followed by any warnings (also capped). Errors from Relate itself are returned as a diagnostic with code `load.database-error` that has no source span.
- A load that succeeds twice on the same file leaves the database as it was after the first (ADR-001 §2.7; SC-003).

## Check

```rust
pub fn check(source: &[u8], settings: &LoadSettings) -> CheckReport;
pub struct CheckReport { pub diagnostics: Vec<Diagnostic> }   // has_errors()
```

Parses and plans `source` as a load would, without a database. It returns the errors and warnings a load would report, in source order, except those that depend on stored elements (`match.*`) and the `load.anonymous` count (ADR-009).

## Render

```rust
pub fn render(d: &Diagnostic, source: &[u8], name: &str) -> String;   // module `render`
```

Renders a diagnostic as plain text: the severity, code and message, the location `name:line:column`, the source line, and a marker under the text at fault, with each related span shown the same way. The text ends with a newline (ADR-009).

## Fix

```rust
pub fn fix(source: &[u8], settings: &LoadSettings, options: &FixOptions) -> Result<FixOutcome, FixFailure>;
pub struct FixOptions { pub rel_type: Option<String>, pub direction: Option<Direction>, pub octal: Option<OctalAs>, pub annotate: bool }
pub struct FixOutcome { pub text: Vec<u8>, pub applied: Vec<Fix>, pub remaining: Vec<Diagnostic>, pub rounds: usize }
```

Corrects the errors that have a mechanical fix (ADR-010, module `fix`) and returns the new text, the fixes applied, and the diagnostics of the new text. It fails if the result would have more errors than the input. Fixing the output again changes nothing. With `annotate`, a `// relate-gram: error[...]` comment is written above each line that still has an error; the function always removes earlier ones (ADR-010 section 2.7).

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
