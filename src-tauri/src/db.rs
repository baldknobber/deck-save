use rusqlite::{Connection, Result};
use std::path::Path;

pub fn init_db(path: &Path) -> Result<Connection> {
    let conn = Connection::open(path)?;
    conn.execute_batch(include_str!("../migrations/001_init.sql"))?;

    // Run migration 002 — add custom_save_paths column (idempotent)
    let has_custom_paths: bool = conn
        .prepare("SELECT sql FROM sqlite_master WHERE type='table' AND name='games'")?
        .query_row([], |row| {
            let sql: String = row.get(0)?;
            Ok(sql.contains("custom_save_paths"))
        })
        .unwrap_or(false);

    if !has_custom_paths {
        conn.execute_batch(include_str!("../migrations/002_custom_save_paths.sql"))?;
    }

    // Run migration 003 — add launcher source column (idempotent)
    let has_launcher: bool = conn
        .prepare("SELECT sql FROM sqlite_master WHERE type='table' AND name='games'")?
        .query_row([], |row| {
            let sql: String = row.get(0)?;
            Ok(sql.contains("launcher"))
        })
        .unwrap_or(false);

    if !has_launcher {
        conn.execute_batch(include_str!("../migrations/003_launcher_source.sql"))?;
    }

    Ok(conn)
}
