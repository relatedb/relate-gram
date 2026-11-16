//! A database for a case: in memory, built from `start.cypher` (ADR-015 section 2.2).
use relate::{Connection, Database};

/// Run Cypher statements, each ending in a semicolon at the end of a line, in file order.
pub fn apply_start(conn: &mut Connection, text: &str) -> Result<(), String> {
    let mut statement = String::new();
    for line in text.lines() {
        if line.trim().is_empty() && statement.is_empty() {
            continue;
        }
        statement.push_str(line);
        statement.push('\n');
        if line.trim_end().ends_with(';') {
            let q = statement.trim().trim_end_matches(';').to_string();
            conn.query(&q)
                .map_err(|e| format!("start.cypher: `{q}`: {e}"))?;
            statement.clear();
        }
    }
    if !statement.trim().is_empty() {
        return Err(format!(
            "start.cypher: a statement does not end in `;`: {}",
            statement.trim()
        ));
    }
    Ok(())
}

/// A fresh in-memory database with the starting statements applied.
pub fn fresh(start: &str) -> Result<(Database, Connection), String> {
    let db = Database::open(":memory:").map_err(|e| e.to_string())?;
    let mut conn = db.connect().map_err(|e| e.to_string())?;
    apply_start(&mut conn, start)?;
    Ok((db, conn))
}
