//! The `relate-gram` command: check, load, export and normalize gram files.

use relate::Database;
use relate_gram::diagnostics::Diagnostic;
use relate_gram::fix::{Direction, FixOptions, OctalAs};
use relate_gram::render::render;
use relate_gram::settings::{ExportSettings, IdentityMode, LoadSettings, TagPolicy};
use std::io::{Read, Write};
use std::process::ExitCode;

const HELP: &str = "\
relate-gram: load gram files into Relate, and export Relate databases as gram

USAGE
    relate-gram check FILE...             Check files without a database
    relate-gram load FILE... --db PATH    Load files into the Relate database at PATH
    relate-gram export --db PATH          Write the database at PATH as gram to stdout
    relate-gram normalize FILE            Print a file as canonical gram (load, then export,
                                          in a scratch in-memory database)
    relate-gram fix FILE [OPTIONS]        Correct the errors of a file that have a mechanical fix;
                                          writes FILE.fixed.gram and leaves FILE alone

FILE may be `-` for standard input.

OPTIONS
    --db PATH                       The Relate database (a directory; created by `load`)
    --node-id-property NAME         Use a node property as identity instead of `@id`
    --relationship-id-property NAME Use a relationship property as identity
    --tags warn|error               A tag on a tagged string: drop and warn (default) or fail
    --identity auto|generate|property
                                    How `export` and `normalize` choose identifiers (default auto)
    --max-errors N                  List at most N errors (default 100)
    -o, --output FILE               Write export or normalize output to FILE instead of stdout
    -q, --quiet                     Do not print warnings
    -h, --help                      Show this help
    -V, --version                   Show the version

FIX OPTIONS
    --rel-type NAME                 Give every untyped relationship the type NAME
    --direction right|left          Point every undirected or bidirectional arrow right (from the
                                    left node to the right node, as written) or left
    --octal decimal|octal           Read an octal literal such as 010 as ten or as eight
    --tags error                    Treat a tag on a tagged string as an error, so it is removed
    -o, --output FILE               Write to FILE instead (`-` for standard output)
    --in-place                      Replace FILE, after copying it to FILE.orig
    --no-backup                     With --in-place, do not copy the original
    --force                         Replace an existing output file or backup
    Without --in-place the original is never changed, and an existing FILE.fixed.gram is never
    replaced. A fix that needs a choice is applied only when its option is given.

EXIT STATUS
    0 success, 1 the files have errors, 2 bad usage or an input/output problem

Behavior is specified in the ADRs under design/adr/ of the relate-gram repository.
";

#[derive(Default)]
struct Options {
    files: Vec<String>,
    db: Option<String>,
    output: Option<String>,
    quiet: bool,
    load: LoadSettings,
    export: ExportSettings,
    fix: FixOptions,
    in_place: bool,
    no_backup: bool,
    force: bool,
}

enum Command {
    Check,
    Load,
    Export,
    Normalize,
    Fix,
}

struct Usage(String);

fn usage<T>(message: impl Into<String>) -> Result<T, Usage> {
    Err(Usage(message.into()))
}

fn value(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<String, Usage> {
    match args.next() {
        Some(v) => Ok(v),
        None => usage(format!("{flag} needs a value")),
    }
}

fn parse(args: Vec<String>) -> Result<Option<(Command, Options)>, Usage> {
    let mut it = args.into_iter();
    let command = match it.next().as_deref() {
        None | Some("-h") | Some("--help") | Some("help") => return Ok(None),
        Some("-V") | Some("--version") => {
            println!("relate-gram {}", env!("CARGO_PKG_VERSION"));
            return Ok(None);
        }
        Some("check") => Command::Check,
        Some("load") => Command::Load,
        Some("export") => Command::Export,
        Some("normalize") => Command::Normalize,
        Some("fix") => Command::Fix,
        Some(other) => return usage(format!("unknown command `{other}`")),
    };
    let mut o = Options::default();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "-h" | "--help" => return Ok(None),
            "--db" => o.db = Some(value(&mut it, "--db")?),
            "-o" | "--output" => o.output = Some(value(&mut it, "--output")?),
            "-q" | "--quiet" => o.quiet = true,
            "--node-id-property" => {
                let v = value(&mut it, &arg)?;
                o.load.node_id_property = Some(v.clone());
                o.export.node_id_property = Some(v);
            }
            "--relationship-id-property" => {
                let v = value(&mut it, &arg)?;
                o.load.relationship_id_property = Some(v.clone());
                o.export.relationship_id_property = Some(v);
            }
            "--tags" => {
                o.load.tags = match value(&mut it, &arg)?.as_str() {
                    "warn" => TagPolicy::Warn,
                    "error" => TagPolicy::Error,
                    other => return usage(format!("--tags takes warn or error, not `{other}`")),
                }
            }
            "--identity" => {
                o.export.identity = match value(&mut it, &arg)?.as_str() {
                    "auto" => IdentityMode::Auto,
                    "generate" => IdentityMode::Generate,
                    "property" => IdentityMode::Property,
                    other => {
                        return usage(format!(
                            "--identity takes auto, generate or property, not `{other}`"
                        ))
                    }
                }
            }
            "--rel-type" => o.fix.rel_type = Some(value(&mut it, &arg)?),
            "--direction" => {
                o.fix.direction = Some(match value(&mut it, &arg)?.as_str() {
                    "right" => Direction::Right,
                    "left" => Direction::Left,
                    other => {
                        return usage(format!("--direction takes right or left, not `{other}`"))
                    }
                })
            }
            "--octal" => {
                o.fix.octal = Some(match value(&mut it, &arg)?.as_str() {
                    "decimal" => OctalAs::Decimal,
                    "octal" => OctalAs::Octal,
                    other => {
                        return usage(format!("--octal takes decimal or octal, not `{other}`"))
                    }
                })
            }
            "--in-place" => o.in_place = true,
            "--no-backup" => o.no_backup = true,
            "--force" => o.force = true,
            "--max-errors" => {
                let v = value(&mut it, &arg)?;
                o.load.max_diagnostics = match v.parse::<usize>() {
                    Ok(n) if n > 0 => n,
                    _ => return usage(format!("--max-errors takes a positive number, not `{v}`")),
                }
            }
            "-" => o.files.push(arg),
            flag if flag.starts_with('-') => return usage(format!("unknown option `{flag}`")),
            _ => o.files.push(arg),
        }
    }
    match command {
        Command::Check if o.files.is_empty() => return usage("check needs at least one FILE"),
        Command::Load if o.files.is_empty() => return usage("load needs at least one FILE"),
        Command::Load | Command::Export if o.db.is_none() => {
            return usage("this command needs --db PATH")
        }
        Command::Normalize if o.files.len() != 1 => {
            return usage("normalize needs exactly one FILE")
        }
        Command::Export if !o.files.is_empty() => return usage("export takes no FILE"),
        Command::Fix if o.files.len() != 1 => return usage("fix needs exactly one FILE"),
        Command::Fix if o.in_place && o.output.is_some() => {
            return usage("--in-place and --output cannot be used together")
        }
        Command::Fix if o.in_place && o.files[0] == "-" => {
            return usage("--in-place needs a file, not standard input")
        }
        _ => {}
    }
    if o.files.iter().filter(|f| *f == "-").count() > 1 {
        return usage("standard input can be given only once");
    }
    Ok(Some((command, o)))
}

fn read(file: &str) -> Result<(String, Vec<u8>), String> {
    if file == "-" {
        let mut bytes = Vec::new();
        std::io::stdin()
            .read_to_end(&mut bytes)
            .map_err(|e| format!("cannot read standard input: {e}"))?;
        Ok(("<stdin>".to_string(), bytes))
    } else {
        let bytes = std::fs::read(file).map_err(|e| format!("cannot read {file}: {e}"))?;
        Ok((file.to_string(), bytes))
    }
}

fn show(diags: &[Diagnostic], source: &[u8], name: &str, quiet: bool) {
    for d in diags {
        if quiet && !d.is_error() {
            continue;
        }
        eprintln!("{}", render(d, source, name));
    }
}

fn count(diags: &[Diagnostic]) -> (usize, usize) {
    let errors = diags.iter().filter(|d| d.is_error()).count();
    (errors, diags.len() - errors)
}

fn emit(text: &str, output: &Option<String>) -> Result<(), String> {
    match output {
        Some(path) => std::fs::write(path, text).map_err(|e| format!("cannot write {path}: {e}")),
        None => std::io::stdout()
            .write_all(text.as_bytes())
            .map_err(|e| e.to_string()),
    }
}

fn open(path: &str) -> Result<Database, String> {
    Database::open(path).map_err(|e| format!("cannot open the database at {path}: {e}"))
}

fn run(command: Command, o: Options) -> Result<ExitCode, String> {
    let mut failed = false;
    match command {
        Command::Check => {
            let (mut errors, mut warnings) = (0, 0);
            for file in &o.files {
                let (name, bytes) = read(file)?;
                let report = relate_gram::check(&bytes, &o.load);
                show(&report.diagnostics, &bytes, &name, o.quiet);
                let (e, w) = count(&report.diagnostics);
                errors += e;
                warnings += w;
            }
            println!(
                "checked {} file(s): {errors} error(s), {warnings} warning(s)",
                o.files.len()
            );
            failed = errors > 0;
        }
        Command::Load => {
            let db = open(o.db.as_deref().unwrap_or_default())?;
            let mut conn = db.connect().map_err(|e| e.to_string())?;
            for file in &o.files {
                let (name, bytes) = read(file)?;
                let mut settings = o.load.clone();
                settings.file_name = Some(name.clone());
                match relate_gram::load(&mut conn, &bytes, &settings) {
                    Ok(report) => {
                        show(&report.diagnostics, &bytes, &name, o.quiet);
                        let c = &report.counts;
                        println!(
                            "{name}: {} node(s) and {} relationship(s) created, {} and {} updated",
                            c.nodes_created,
                            c.relationships_created,
                            c.nodes_updated,
                            c.relationships_updated
                        );
                    }
                    Err(f) => {
                        show(&f.diagnostics, &bytes, &name, o.quiet);
                        eprintln!(
                            "{name}: not loaded ({} error(s)); the database is unchanged\n",
                            f.total
                        );
                        failed = true;
                        break;
                    }
                }
            }
        }
        Command::Export => {
            let db = open(o.db.as_deref().unwrap_or_default())?;
            let mut conn = db.connect().map_err(|e| e.to_string())?;
            match relate_gram::export(&mut conn, &o.export) {
                Ok(text) => emit(&text, &o.output)?,
                Err(f) => {
                    show(&f.diagnostics, b"", "", false);
                    eprintln!("not exported ({} error(s))", f.total);
                    failed = true;
                }
            }
        }
        Command::Fix => return run_fix(o),
        Command::Normalize => {
            let (name, bytes) = read(&o.files[0])?;
            let db = Database::open(":memory:").map_err(|e| e.to_string())?;
            let mut conn = db.connect().map_err(|e| e.to_string())?;
            let mut settings = o.load.clone();
            settings.file_name = Some(name.clone());
            match relate_gram::load(&mut conn, &bytes, &settings) {
                Ok(report) => {
                    show(&report.diagnostics, &bytes, &name, o.quiet);
                    match relate_gram::export(&mut conn, &o.export) {
                        Ok(text) => emit(&text, &o.output)?,
                        Err(f) => {
                            show(&f.diagnostics, b"", "", false);
                            failed = true;
                        }
                    }
                }
                Err(f) => {
                    show(&f.diagnostics, &bytes, &name, o.quiet);
                    failed = true;
                }
            }
        }
    }
    Ok(if failed {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    })
}

/// `people.gram` becomes `people.fixed.gram`; any other name gets `.fixed` appended.
fn fixed_name(path: &str) -> String {
    match path.strip_suffix(".gram") {
        Some(stem) => format!("{stem}.fixed.gram"),
        None => format!("{path}.fixed"),
    }
}

/// Write `text` to `path` through a temporary file in the same directory, keeping
/// the permissions of `like` if given.
fn write_atomically(
    path: &std::path::Path,
    text: &[u8],
    like: Option<&std::fs::Metadata>,
) -> Result<(), String> {
    let dir = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(std::path::Path::new("."));
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let tmp = dir.join(format!(".{name}.relate-gram-{}.tmp", std::process::id()));
    std::fs::write(&tmp, text).map_err(|e| format!("cannot write {}: {e}", tmp.display()))?;
    if let Some(meta) = like {
        let _ = std::fs::set_permissions(&tmp, meta.permissions());
    }
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        format!("cannot write {}: {e}", path.display())
    })
}

fn run_fix(o: Options) -> Result<ExitCode, String> {
    let file = &o.files[0];
    let (name, bytes) = read(file)?;
    let outcome = match relate_gram::fix::fix(&bytes, &o.load, &o.fix) {
        Ok(outcome) => outcome,
        Err(f) => {
            eprintln!("{name}: {f}");
            return Ok(ExitCode::from(1));
        }
    };

    let stdout = file == "-" || o.output.as_deref() == Some("-");
    let output_path = if stdout {
        None
    } else if o.in_place {
        Some(std::fs::canonicalize(file).map_err(|e| format!("cannot resolve {file}: {e}"))?)
    } else {
        Some(std::path::PathBuf::from(
            o.output.clone().unwrap_or_else(|| fixed_name(file)),
        ))
    };

    if outcome.applied.is_empty() {
        show(&outcome.remaining, &bytes, &name, o.quiet);
        if outcome.has_errors() {
            hint(&outcome.remaining, &o);
            eprintln!("{name}: no fix applies; nothing was written");
            return Ok(ExitCode::from(1));
        }
        eprintln!("{name}: no errors; nothing to fix");
        return Ok(ExitCode::SUCCESS);
    }

    let mut shown_as = "standard output".to_string();
    if let Some(path) = &output_path {
        shown_as = path.display().to_string();
        if o.output.is_none() && !o.in_place && path.exists() && !o.force {
            return Err(format!(
                "{} already exists; use -o to name another file, or --force to replace it",
                path.display()
            ));
        }
        if o.in_place && !o.no_backup {
            let backup = std::path::PathBuf::from(format!("{}.orig", path.display()));
            if backup.exists() && !o.force {
                return Err(format!(
                    "{} already exists; use --no-backup to skip the copy, or --force to replace it",
                    backup.display()
                ));
            }
            std::fs::write(&backup, &bytes)
                .map_err(|e| format!("cannot write {}: {e}", backup.display()))?;
        }
        let like = if o.in_place {
            std::fs::metadata(path).ok()
        } else {
            None
        };
        write_atomically(path, &outcome.text, like.as_ref())?;
    } else {
        std::io::stdout()
            .write_all(&outcome.text)
            .map_err(|e| e.to_string())?;
    }

    let mut by_code: Vec<(&str, usize, String)> = Vec::new();
    for f in &outcome.applied {
        match by_code.iter_mut().find(|(c, _, _)| *c == f.code) {
            Some(entry) => entry.1 += 1,
            None => by_code.push((f.code, 1, f.description.clone())),
        }
    }
    eprintln!(
        "{name}: applied {} fix(es) in {} round(s):",
        outcome.applied.len(),
        outcome.rounds
    );
    for (code, n, what) in &by_code {
        eprintln!("  {n} x {code}: {what}");
    }
    eprintln!("wrote {shown_as}");
    let label = if output_path.is_some() {
        shown_as.as_str()
    } else {
        name.as_str()
    };
    show(&outcome.remaining, &outcome.text, label, o.quiet);
    if outcome.has_errors() {
        hint(&outcome.remaining, &o);
        eprintln!(
            "{label}: {} error(s) remain",
            outcome.remaining.iter().filter(|d| d.is_error()).count()
        );
        return Ok(ExitCode::from(1));
    }
    Ok(ExitCode::SUCCESS)
}

/// Say which options would make the remaining errors fixable.
fn hint(remaining: &[Diagnostic], o: &Options) {
    let given = |code: &str| match code {
        "rel.no-type" => o.fix.rel_type.is_some(),
        "rel.undirected" | "rel.bidirectional" => o.fix.direction.is_some(),
        "value.octal" => o.fix.octal.is_some(),
        _ => true,
    };
    let mut seen: Vec<(&str, &str, usize)> = Vec::new();
    for d in remaining.iter().filter(|d| d.is_error()) {
        if let (Some(opt), false) = (relate_gram::fix::needs(d.code), given(d.code)) {
            match seen.iter_mut().find(|(c, _, _)| *c == d.code) {
                Some(e) => e.2 += 1,
                None => seen.push((d.code, opt, 1)),
            }
        }
    }
    for (code, opt, n) in seen {
        eprintln!("hint: {n} error(s) of {code} can be fixed with {opt}");
    }
}

fn main() -> ExitCode {
    match parse(std::env::args().skip(1).collect()) {
        Ok(None) => {
            print!("{HELP}");
            ExitCode::SUCCESS
        }
        Ok(Some((command, options))) => match run(command, options) {
            Ok(code) => code,
            Err(message) => {
                eprintln!("relate-gram: {message}");
                ExitCode::from(2)
            }
        },
        Err(Usage(message)) => {
            eprintln!("relate-gram: {message}\n\nRun `relate-gram --help` for usage.");
            ExitCode::from(2)
        }
    }
}
