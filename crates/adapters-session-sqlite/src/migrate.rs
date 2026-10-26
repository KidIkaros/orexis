//! Schema creation and migration.

use orexis_ports::PortResult;
use rusqlite::{params, Connection, OptionalExtension};

use crate::util::err;

const CURRENT_SCHEMA: i64 = 2;

pub(crate) fn migrate(conn: &Connection) -> PortResult<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
         );
         CREATE TABLE IF NOT EXISTS session (
            id TEXT PRIMARY KEY,
            parent_id TEXT,
            fork_seq INTEGER,
            format_version INTEGER NOT NULL,
            profile_id TEXT,
            model_id TEXT,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
         );
         CREATE TABLE IF NOT EXISTS event (
            session_id TEXT NOT NULL,
            seq INTEGER NOT NULL,
            event_id TEXT NOT NULL,
            kind TEXT NOT NULL,
            version INTEGER NOT NULL,
            generation_id INTEGER NOT NULL,
            payload_json TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            PRIMARY KEY (session_id, seq)
         );
         CREATE TABLE IF NOT EXISTS generation (
            session_id TEXT NOT NULL,
            generation_id INTEGER NOT NULL,
            starts_at_seq INTEGER NOT NULL,
            reason TEXT NOT NULL,
            PRIMARY KEY (session_id, generation_id)
         );",
    )
    .map_err(err)?;

    let version: i64 = conn
        .query_row(
            "SELECT value FROM meta WHERE key = 'schema_version'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(err)?
        .and_then(|value| value.parse().ok())
        .unwrap_or(0);

    if version < CURRENT_SCHEMA {
        if version == 1 && !has_column(conn, "event", "generation_id")? {
            conn.execute(
                "ALTER TABLE event ADD COLUMN generation_id INTEGER NOT NULL DEFAULT 1",
                [],
            )
            .map_err(err)?;
        }
        conn.execute(
            "INSERT INTO meta (key, value) VALUES ('schema_version', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![CURRENT_SCHEMA.to_string()],
        )
        .map_err(err)?;
    }
    Ok(())
}

fn has_column(conn: &Connection, table: &str, column: &str) -> PortResult<bool> {
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(err)?;
    let mut rows = stmt.query([]).map_err(err)?;
    while let Some(row) = rows.next().map_err(err)? {
        let name: String = row.get(1).map_err(err)?;
        if name == column {
            return Ok(true);
        }
    }
    Ok(false)
}
