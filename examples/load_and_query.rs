//! Load a gram file into an in-memory Relate database and print what it holds.
//!
//! Usage: `cargo run --example load_and_query -- path/to/file.gram`

use relate::{Database, Value};
use relate_gram::settings::LoadSettings;
use std::process::ExitCode;

fn count(conn: &mut relate::Connection, cypher: &str) -> i64 {
    match conn.query(cypher).map(|r| r.rows) {
        Ok(rows) => match rows.first().and_then(|r| r.first()) {
            Some(Value::Int(i)) => *i,
            _ => -1,
        },
        Err(_) => -1,
    }
}

fn main() -> ExitCode {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: load_and_query <file.gram>");
        return ExitCode::from(2);
    };
    let source = match std::fs::read(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("cannot read {path}: {e}");
            return ExitCode::from(2);
        }
    };
    let db = Database::open(":memory:").expect("open an in-memory database");
    let mut conn = db.connect().expect("connect");
    let settings = LoadSettings {
        file_name: Some(path.clone()),
        ..LoadSettings::default()
    };
    match relate_gram::load(&mut conn, &source, &settings) {
        Ok(report) => {
            for d in &report.diagnostics {
                println!("{d}");
            }
            println!(
                "loaded {} node(s) and {} relationship(s)",
                count(&mut conn, "MATCH (n) RETURN count(n)"),
                count(&mut conn, "MATCH ()-[r]->() RETURN count(r)")
            );
            ExitCode::SUCCESS
        }
        Err(failure) => {
            for d in &failure.diagnostics {
                eprintln!("{d}");
            }
            eprintln!("{} diagnostic(s); the database is unchanged", failure.total);
            ExitCode::FAILURE
        }
    }
}
