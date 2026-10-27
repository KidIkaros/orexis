//! Schema creation and migration.

use orexis_ports::PortResult;
use rusqlite::Connection;

use crate::util::err;

pub(crate) const CURRENT_SCHEMA: i64 = 1;

pub(crate) fn migrate(conn: &Connection) -> PortResult<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
         );
         CREATE TABLE IF NOT EXISTS task (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            required_strategy TEXT NOT NULL,
            owner TEXT,
            status TEXT NOT NULL,
            seq INTEGER NOT NULL UNIQUE,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
         );
         CREATE TABLE IF NOT EXISTS task_edge (
            task_id TEXT NOT NULL REFERENCES task(id),
            depends_on TEXT NOT NULL,
            PRIMARY KEY (task_id, depends_on)
         );",
    )
    .map_err(err)?;
    let version: i64 = conn
        .query_row(
            "SELECT COALESCE((SELECT value FROM meta WHERE key = 'schema'), '0')",
            [],
            |row| row.get::<_, String>(0),
        )
        .map_err(err)?
        .parse::<i64>()
        .unwrap_or(0);
    if version > CURRENT_SCHEMA {
        return Err(orexis_ports::PortError(format!(
            "board schema {version} is newer than this build knows ({CURRENT_SCHEMA})"
        )));
    }
    if version < CURRENT_SCHEMA {
        conn.execute(
            "INSERT INTO meta (key, value) VALUES ('schema', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            rusqlite::params![CURRENT_SCHEMA.to_string()],
        )
        .map_err(err)?;
    }
    Ok(())
}
