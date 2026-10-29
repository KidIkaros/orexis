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
            .prepare("SELECT id FROM session ORDER BY created_at, rowid")
            .map_err(err)?;
        let rows = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(err)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(err)
    }

    pub fn export_all_jsonl(path: &Path) -> PortResult<String> {
        if !path.exists() {
            return Ok(String::new());
        }
        let conn = Connection::open(path).map_err(err)?;
        migrate(&conn)?;
        let mut out = String::new();

        let mut s_stmt = conn
            .prepare(
                "SELECT id, parent_id, fork_seq, format_version, profile_id, model_id, created_at, updated_at
                 FROM session ORDER BY created_at, rowid",
            )
            .map_err(err)?;
        let s_rows = s_stmt
            .query_map([], |row| {
                Ok(serde_json::json!({
                    "record": "session",
                    "id": row.get::<_, String>(0)?,
                    "parent_id": row.get::<_, Option<String>>(1)?,
                    "fork_seq": row.get::<_, Option<i64>>(2)?,
                    "format_version": row.get::<_, i64>(3)?,
                    "profile_id": row.get::<_, Option<String>>(4)?,
                    "model_id": row.get::<_, Option<String>>(5)?,
                    "created_at": row.get::<_, i64>(6)?,
                    "updated_at": row.get::<_, i64>(7)?,
                }))
            })
            .map_err(err)?;
        for row in s_rows {
            let val = row.map_err(err)?;
            out.push_str(&val.to_string());
            out.push('\n');
        }

        let mut g_stmt = conn
            .prepare(
                "SELECT session_id, generation_id, starts_at_seq, reason
                 FROM generation ORDER BY rowid",
            )
            .map_err(err)?;
        let g_rows = g_stmt
            .query_map([], |row| {
                Ok(serde_json::json!({
                    "record": "generation",
                    "session_id": row.get::<_, String>(0)?,
                    "generation_id": row.get::<_, i64>(1)?,
                    "starts_at_seq": row.get::<_, i64>(2)?,
                    "reason": row.get::<_, String>(3)?,
                }))
            })
            .map_err(err)?;
        for row in g_rows {
            let val = row.map_err(err)?;
            out.push_str(&val.to_string());
            out.push('\n');
        }

        let mut e_stmt = conn
            .prepare(
                "SELECT session_id, seq, event_id, generation_id, payload_json
                 FROM event ORDER BY rowid, seq",
            )
            .map_err(err)?;
        let e_rows = e_stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })
            .map_err(err)?;
        for row in e_rows {
            let (session_id, seq, event_id, generation_id, payload) = row.map_err(err)?;
            out.push_str(&format!(
                "{{\"record\":\"event\",\"session_id\":\"{session_id}\",\"seq\":{seq},\"event_id\":\"{event_id}\",\"generation_id\":{generation_id},\"event\":{payload}}}\n"
            ));
        }

        Ok(out)
    }

    pub fn is_session_record(line: &str) -> PortResult<bool> {
        let val: serde_json::Value =
            serde_json::from_str(line).map_err(|e| PortError(e.to_string()))?;
        Ok(val.get("record").is_some() || val.get("event_id").is_some())
    }

    pub fn import_all_jsonl(path: &Path, jsonl: &str) -> PortResult<usize> {
        let conn = Connection::open(path).map_err(err)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;")
            .map_err(err)?;
        migrate(&conn)?;
        let mut imported_events = 0usize;
        for line in jsonl.lines().filter(|l| !l.trim().is_empty()) {
            let value: serde_json::Value =
                serde_json::from_str(line).map_err(|e| PortError(e.to_string()))?;
            match value.get("record").and_then(|v| v.as_str()) {
                Some("session") => {
                    let id = value["id"]
                        .as_str()
                        .ok_or_else(|| PortError("session record missing id".into()))?;
                    let parent_id = value["parent_id"].as_str();
                    let fork_seq = value["fork_seq"].as_i64();
                    let format_version = value["format_version"]
                        .as_i64()
                        .unwrap_or(FORMAT_VERSION as i64);
                    let profile_id = value["profile_id"].as_str();
                    let model_id = value["model_id"].as_str();
                    let created_at = value["created_at"].as_i64().unwrap_or(0);
                    let updated_at = value["updated_at"].as_i64().unwrap_or(0);
                    conn.execute(
                        "INSERT OR REPLACE INTO session (id, parent_id, fork_seq, format_version, profile_id, model_id, created_at, updated_at)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                        params![
                            id,
                            parent_id,
                            fork_seq,
                            format_version,
                            profile_id,
                            model_id,
                            created_at,
                            updated_at
                        ],
                    )
                    .map_err(err)?;
                }
                Some("generation") => {
                    let session_id = value["session_id"]
                        .as_str()
                        .ok_or_else(|| PortError("generation record missing session_id".into()))?;
                    let generation_id = value["generation_id"].as_i64().unwrap_or(1);
                    let starts_at_seq = value["starts_at_seq"].as_i64().unwrap_or(1);
                    let reason = value["reason"].as_str().unwrap_or("start");
                    conn.execute(
                        "INSERT OR REPLACE INTO generation (session_id, generation_id, starts_at_seq, reason)
                         VALUES (?1, ?2, ?3, ?4)",
                        params![session_id, generation_id, starts_at_seq, reason],
                    )
                    .map_err(err)?;
                }
                Some("event") | None => {
                    let session_id = value
                        .get("session_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("imported");
                    conn.execute(
                        "INSERT OR IGNORE INTO session (id, parent_id, fork_seq, format_version, profile_id, model_id, created_at, updated_at)
                         VALUES (?1, NULL, NULL, ?2, NULL, NULL, unixepoch(), unixepoch())",
                        params![session_id, FORMAT_VERSION as i64],
                    )
                    .map_err(err)?;
                    let seq = value["seq"]
                        .as_i64()
                        .ok_or_else(|| PortError("event record missing seq".into()))?;
                    let event_id = value
                        .get("event_id")
                        .and_then(|v| v.as_str())
                        .map(str::to_string)
                        .unwrap_or_else(new_id);
                    let generation_id = value
                        .get("generation_id")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(1);
                    let event: SessionEvent = serde_json::from_value(value["event"].clone())
                        .map_err(|e| PortError(e.to_string()))?;
                    let event = redact_event(&event);
                    let payload =
                        serde_json::to_string(&event).map_err(|e| PortError(e.to_string()))?;
                    conn.execute(
                        "INSERT OR REPLACE INTO event (session_id, seq, event_id, kind, version, generation_id, payload_json, created_at)
                         VALUES (?1, ?2, ?3, ?4, 1, ?5, ?6, unixepoch())",
                        params![session_id, seq, event_id, event.kind(), generation_id, payload],
                    )
                    .map_err(err)?;
                    imported_events += 1;
                }
                _ => {}
            }
        }
        Ok(imported_events)
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
