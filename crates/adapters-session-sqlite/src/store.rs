//! The session store and its ports.

use std::path::Path;
use std::sync::Mutex;

use async_trait::async_trait;
use orexis_core::{redact_event, EventRef, SessionEvent, StoredEvent, FORMAT_VERSION};
use orexis_ports::{PortError, PortResult, SessionAppend, SessionFork, SessionRead};
use rusqlite::{params, Connection, OptionalExtension};

use crate::migrate::migrate;
use crate::util::{err, lock, new_id};

pub struct SqliteSessionStore {
    conn: Mutex<Connection>,
    session_id: String,
}

impl SqliteSessionStore {
    /// Open a database and create a new session.
    pub fn open(path: &Path) -> PortResult<Self> {
        let conn = Connection::open(path).map_err(err)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;")
            .map_err(err)?;
        migrate(&conn)?;
        let session_id = new_id();
        conn.execute(
            "INSERT INTO session (id, parent_id, fork_seq, format_version, profile_id, model_id, created_at, updated_at)
             VALUES (?1, NULL, NULL, ?2, NULL, NULL, unixepoch(), unixepoch())",
            params![session_id, FORMAT_VERSION as i64],
        )
        .map_err(err)?;
        conn.execute(
            "INSERT INTO generation (session_id, generation_id, starts_at_seq, reason)
             VALUES (?1, 1, 1, 'start')",
            params![session_id],
        )
        .map_err(err)?;
        Ok(Self {
            conn: Mutex::new(conn),
            session_id,
        })
    }

    /// Open a database and attach to an existing session.
    pub fn open_session(path: &Path, session_id: &str) -> PortResult<Self> {
        let conn = Connection::open(path).map_err(err)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;")
            .map_err(err)?;
        migrate(&conn)?;
        let exists: Option<String> = conn
            .query_row(
                "SELECT id FROM session WHERE id = ?1",
                params![session_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(err)?;
        if exists.is_none() {
            return Err(PortError(format!("unknown session {session_id}")));
        }
        Ok(Self {
            conn: Mutex::new(conn),
            session_id: session_id.to_string(),
        })
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn fork_at(&self, at_seq: u64) -> PortResult<String> {
        let conn = self.conn.lock().map_err(lock)?;
        let new_session = new_id();
        conn.execute(
            "INSERT INTO session (id, parent_id, fork_seq, format_version, profile_id, model_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, NULL, NULL, unixepoch(), unixepoch())",
            params![
                new_session,
                self.session_id,
                at_seq as i64,
                FORMAT_VERSION as i64
            ],
        )
        .map_err(err)?;
        conn.execute(
            "INSERT INTO generation (session_id, generation_id, starts_at_seq, reason)
             VALUES (?1, 1, 1, 'fork')",
            params![new_session],
        )
        .map_err(err)?;

        let copied = {
            let mut stmt = conn
                .prepare(
                    "SELECT seq, kind, generation_id, payload_json
                     FROM event WHERE session_id = ?1 AND seq <= ?2 ORDER BY seq",
                )
                .map_err(err)?;
            let rows = stmt
                .query_map(params![self.session_id, at_seq as i64], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                })
                .map_err(err)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(err)?
        };

        for (seq, kind, generation_id, payload) in copied {
            conn.execute(
                "INSERT INTO event (session_id, seq, event_id, kind, version, generation_id, payload_json, created_at)
                 VALUES (?1, ?2, ?3, ?4, 1, ?5, ?6, unixepoch())",
                params![new_session, seq, new_id(), kind, generation_id, payload],
            )
            .map_err(err)?;
        }

        Ok(new_session)
    }

    pub fn export_jsonl(&self) -> PortResult<String> {
        let conn = self.conn.lock().map_err(lock)?;
        let mut stmt = conn
            .prepare(
                "SELECT seq, event_id, generation_id, payload_json
                 FROM event WHERE session_id = ?1 ORDER BY seq",
            )
            .map_err(err)?;
        let rows = stmt
            .query_map(params![self.session_id], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .map_err(err)?;
        let mut out = String::new();
        for row in rows {
            let (seq, event_id, generation_id, payload) = row.map_err(err)?;
            out.push_str(&format!(
                "{{\"seq\":{seq},\"event_id\":\"{event_id}\",\"generation_id\":{generation_id},\"event\":{payload}}}\n"
            ));
        }
        Ok(out)
    }

    pub fn list_sessions(path: &Path) -> PortResult<Vec<String>> {
        if !path.exists() {
            return Ok(Vec::new());
        }
        let conn = Connection::open(path).map_err(err)?;
        migrate(&conn)?;
        let mut stmt = conn
            .prepare("SELECT id FROM session ORDER BY created_at")
            .map_err(err)?;
        let rows = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(err)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(err)
    }

    pub fn migrate_now(&self) -> PortResult<()> {
        let conn = self.conn.lock().map_err(lock)?;
        migrate(&conn)
    }
}

#[async_trait]
impl SessionAppend for SqliteSessionStore {
    async fn append(&self, event: SessionEvent) -> PortResult<EventRef> {
        let event = redact_event(&event);
        let conn = self.conn.lock().map_err(lock)?;
        let seq: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(seq), 0) + 1 FROM event WHERE session_id = ?1",
                params![self.session_id],
                |row| row.get(0),
            )
            .map_err(err)?;
        let event_id = new_id();
        let kind = event.kind();
        let payload = serde_json::to_string(&event).map_err(|e| PortError(e.to_string()))?;
        conn.execute(
            "INSERT INTO event (session_id, seq, event_id, kind, version, generation_id, payload_json, created_at)
             VALUES (?1, ?2, ?3, ?4, 1, 1, ?5, unixepoch())",
            params![self.session_id, seq, event_id, kind, payload],
        )
        .map_err(err)?;
        conn.execute(
            "UPDATE session SET updated_at = unixepoch() WHERE id = ?1",
            params![self.session_id],
        )
        .map_err(err)?;
        Ok(EventRef {
            seq: seq as u64,
            event_id,
        })
    }
}

#[async_trait]
impl SessionRead for SqliteSessionStore {
    async fn events(&self) -> PortResult<Vec<StoredEvent>> {
        let conn = self.conn.lock().map_err(lock)?;
        let mut stmt = conn
            .prepare(
                "SELECT seq, event_id, generation_id, payload_json
                 FROM event WHERE session_id = ?1 ORDER BY seq",
            )
            .map_err(err)?;
        let rows = stmt
            .query_map(params![self.session_id], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .map_err(err)?;
        let mut out = Vec::new();
        for row in rows {
            let (seq, event_id, generation_id, payload) = row.map_err(err)?;
            let event: SessionEvent =
                serde_json::from_str(&payload).map_err(|e| PortError(e.to_string()))?;
            out.push(StoredEvent {
                seq: seq as u64,
                event_id,
                generation_id: generation_id as u64,
                event,
            });
        }
        Ok(out)
    }
}

#[async_trait]
impl SessionFork for SqliteSessionStore {
    async fn fork(&self, at_seq: u64) -> PortResult<String> {
        self.fork_at(at_seq)
    }
}
