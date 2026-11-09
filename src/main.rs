//! The `relate-gram` command: check, load, export and normalize gram files.

use relate::Database;
use relate_gram::diagnostics::Diagnostic;
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
}

enum Command {
    Check,
    Load,
    Export,
    Normalize,
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
