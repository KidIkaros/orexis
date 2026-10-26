//! SQLite session store implementing ADR-0001.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use orexis_core::{redact_event, EventRef, SessionEvent, StoredEvent, FORMAT_VERSION};
use orexis_ports::{
    PortError, PortResult, SessionAppend, SessionFactory, SessionFork, SessionRead, SessionRefs,
};
use rusqlite::{params, Connection, OptionalExtension};

const CURRENT_SCHEMA: i64 = 2;

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

fn migrate(conn: &Connection) -> PortResult<()> {
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

fn new_id() -> String {
    ulid::Ulid::new().to_string()
}

fn err(error: rusqlite::Error) -> PortError {
    PortError(error.to_string())
}

fn lock<T>(_: T) -> PortError {
    PortError("store lock poisoned".into())
}

pub struct SqliteSessionFactory {
    db: PathBuf,
}

impl SqliteSessionFactory {
    pub fn new(db: impl Into<PathBuf>) -> Self {
        Self { db: db.into() }
    }
}

impl SessionFactory for SqliteSessionFactory {
    fn open(&self, session: Option<&str>) -> PortResult<SessionRefs> {
        let store = match session {
            Some(id) => Arc::new(SqliteSessionStore::open_session(&self.db, id)?),
            None => Arc::new(SqliteSessionStore::open(&self.db)?),
        };
        let append: Arc<dyn SessionAppend> = store.clone();
        let read: Arc<dyn SessionRead> = store.clone();
        Ok(SessionRefs {
            id: store.session_id().to_string(),
            append,
            read,
        })
    }

    fn list(&self) -> PortResult<Vec<String>> {
        SqliteSessionStore::list_sessions(&self.db)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::executor::block_on;

    fn user(text: &str) -> SessionEvent {
        SessionEvent::UserMessage {
            text: text.to_string(),
        }
    }

    #[test]
    fn append_and_read_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let store = SqliteSessionStore::open(&dir.path().join("s.db")).unwrap();
        block_on(store.append(user("one"))).unwrap();
        block_on(store.append(user("two"))).unwrap();

        let events = block_on(store.events()).unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].seq, 1);
        assert_eq!(events[1].seq, 2);
        assert_eq!(events[0].generation_id, 1);
        assert!(events[0].event_id.len() > 10);
    }

    #[test]
    fn reopen_preserves_events() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("s.db");
        let id = {
            let store = SqliteSessionStore::open(&path).unwrap();
            block_on(store.append(user("one"))).unwrap();
            store.session_id().to_string()
        };
        let reopened = SqliteSessionStore::open_session(&path, &id).unwrap();
        let events = block_on(reopened.events()).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].seq, 1);
    }

    #[test]
    fn fork_copies_the_prefix() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("s.db");
        let store = SqliteSessionStore::open(&path).unwrap();
        for text in ["one", "two", "three"] {
            block_on(store.append(user(text))).unwrap();
        }
        let forked = store.fork_at(2).unwrap();
        assert_ne!(forked, store.session_id());

        let child = SqliteSessionStore::open_session(&path, &forked).unwrap();
        let events = block_on(child.events()).unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[1].seq, 2);

        let parent_events = block_on(store.events()).unwrap();
        assert_eq!(parent_events.len(), 3);
    }

    #[test]
    fn redacts_at_write() {
        let dir = tempfile::tempdir().unwrap();
        let store = SqliteSessionStore::open(&dir.path().join("s.db")).unwrap();
        block_on(store.append(user("use sk-abcdefghijklmnopqrstuvwx now"))).unwrap();
        let events = block_on(store.events()).unwrap();
        let SessionEvent::UserMessage { text } = &events[0].event else {
            panic!("wrong variant");
        };
        assert!(text.contains("[REDACTED:"));
        assert!(!text.contains("sk-abcdefghijklmnopqrstuvwx"));
    }

    #[test]
    fn export_contains_every_event() {
        let dir = tempfile::tempdir().unwrap();
        let store = SqliteSessionStore::open(&dir.path().join("s.db")).unwrap();
        block_on(store.append(user("one"))).unwrap();
        let jsonl = store.export_jsonl().unwrap();
        assert!(jsonl.contains("\"seq\":1"));
        assert!(jsonl.contains("user_message"));
    }

    #[test]
    fn migrates_a_v1_event_table() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("v1.db");
        {
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch(
                "CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
                 INSERT INTO meta VALUES ('schema_version', '1');
                 CREATE TABLE session (
                    id TEXT PRIMARY KEY, parent_id TEXT, fork_seq INTEGER,
                    format_version INTEGER NOT NULL, profile_id TEXT, model_id TEXT,
                    created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL
                 );
                 CREATE TABLE event (
                    session_id TEXT NOT NULL, seq INTEGER NOT NULL, event_id TEXT NOT NULL,
                    kind TEXT NOT NULL, version INTEGER NOT NULL,
                    payload_json TEXT NOT NULL, created_at INTEGER NOT NULL,
                    PRIMARY KEY (session_id, seq)
                 );
                 CREATE TABLE generation (
                    session_id TEXT NOT NULL, generation_id INTEGER NOT NULL,
                    starts_at_seq INTEGER NOT NULL, reason TEXT NOT NULL,
                    PRIMARY KEY (session_id, generation_id)
                 );
                 INSERT INTO session VALUES ('s1', NULL, NULL, 1, NULL, NULL, 0, 0);
                 INSERT INTO generation VALUES ('s1', 1, 1, 'start');",
            )
            .unwrap();
            let payload = serde_json::to_string(&user("old")).unwrap();
            conn.execute(
                "INSERT INTO event (session_id, seq, event_id, kind, version, payload_json, created_at)
                 VALUES ('s1', 1, 'e1', 'user_message', 1, ?1, 0)",
                params![payload],
            )
            .unwrap();
        }

        let store = SqliteSessionStore::open_session(&path, "s1").unwrap();
        let events = block_on(store.events()).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].generation_id, 1);
    }
}
