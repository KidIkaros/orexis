//! The bus store and its port.

use std::path::Path;
use std::sync::Mutex;

use async_trait::async_trait;
use orexis_core::BusMessage;
use orexis_ports::{MessageStore, PortResult};
use rusqlite::{params, Connection};

use crate::util::{err, lock};

/// Bus text is capped — unbounded messages are an anti-pattern.
const TEXT_CAP: usize = 64 * 1024;

pub struct SqliteBus {
    conn: Mutex<Connection>,
}

impl SqliteBus {
    pub fn open(path: &Path) -> PortResult<Self> {
        let conn = Connection::open(path).map_err(err)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;")
            .map_err(err)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS message (
                seq INTEGER PRIMARY KEY,
                from_w TEXT NOT NULL,
                to_w TEXT,
                text TEXT NOT NULL,
                created_at INTEGER NOT NULL
             );",
        )
        .map_err(err)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }
}

#[async_trait]
impl MessageStore for SqliteBus {
    async fn post(&self, from: &str, to: Option<&str>, text: &str) -> PortResult<BusMessage> {
        let conn = self.conn.lock().map_err(lock)?;
        let capped: String = text.chars().take(TEXT_CAP).collect();
        conn.execute(
            "INSERT INTO message (from_w, to_w, text, created_at)
             VALUES (?1, ?2, ?3, unixepoch())",
            params![from, to, capped],
        )
        .map_err(err)?;
        let seq = conn.last_insert_rowid() as u64;
        Ok(BusMessage {
            seq,
            from: from.to_string(),
            to: to.map(str::to_string),
            text: capped,
        })
    }

    async fn read_since(&self, seq: u64, for_worker: Option<&str>) -> PortResult<Vec<BusMessage>> {
        let conn = self.conn.lock().map_err(lock)?;
        let mut statement = conn
            .prepare(
                "SELECT seq, from_w, to_w, text FROM message
                 WHERE seq > ?1 AND (?2 IS NULL OR to_w IS NULL OR to_w = ?2)
                 ORDER BY seq",
            )
            .map_err(err)?;
        let rows = statement
            .query_map(params![seq as i64, for_worker], |row| {
                Ok(BusMessage {
                    seq: row.get(0)?,
                    from: row.get(1)?,
                    to: row.get(2)?,
                    text: row.get(3)?,
                })
            })
            .map_err(err)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(err);
        drop(statement);
        rows
    }
}
