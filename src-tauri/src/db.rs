use rusqlite::{Connection, Result};
use std::path::Path;

pub fn init_db(path: &Path) -> Result<Connection> {
    let conn = Connection::open(path)?;
    conn.execute_batch(include_str!("../migrations/001_init.sql"))?;
    Ok(conn)
}
