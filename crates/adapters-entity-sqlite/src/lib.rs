//! SQLite entity store: an append-only log plus derived projections.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use orexis_core::{
    project, redact_entity_event, EntityEvent, EntitySnapshot, ENTITY_FORMAT_VERSION,
};
use orexis_ports::{
    EntityAppend, EntityFactory, EntityRead, EntityRefs, PortError, PortResult, StoredEntityEvent,
};
use rusqlite::{params, Connection, OptionalExtension};

pub struct SqliteEntityStore {
    conn: Mutex<Connection>,
    entity_id: String,
}

impl SqliteEntityStore {
    pub fn entity_id(&self) -> &str {
        &self.entity_id
    }

    pub fn open(path: &Path) -> PortResult<Self> {
        let conn = Connection::open(path).map_err(err)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;")
            .map_err(err)?;
        migrate(&conn)?;
        let existing: Option<String> = conn
            .query_row(
                "SELECT payload_json FROM entity_event WHERE kind = 'entity_start' ORDER BY seq LIMIT 1",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(err)?;
        let entity_id = match existing {
            Some(payload) => match serde_json::from_str::<EntityEvent>(&payload) {
                Ok(EntityEvent::EntityStart { entity_id, .. }) => entity_id,
                _ => new_id(),
            },
            None => {
                let id = new_id();
                insert_event(
                    &conn,
                    &EntityEvent::EntityStart {
                        entity_id: id.clone(),
                        format_version: ENTITY_FORMAT_VERSION,
                    },
                )?;
                id
            }
        };
        Ok(Self {
            conn: Mutex::new(conn),
            entity_id,
        })
    }
}

#[async_trait]
impl EntityAppend for SqliteEntityStore {
    async fn append(&self, event: EntityEvent) -> PortResult<u64> {
        let conn = self.conn.lock().map_err(lock)?;
        insert_event(&conn, &event)
    }
}

#[async_trait]
impl EntityRead for SqliteEntityStore {
    async fn events(&self) -> PortResult<Vec<StoredEntityEvent>> {
        let conn = self.conn.lock().map_err(lock)?;
        let mut stmt = conn
            .prepare("SELECT seq, payload_json FROM entity_event ORDER BY seq")
            .map_err(err)?;
        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(err)?;
        let mut out = Vec::new();
        for row in rows {
            let (seq, payload) = row.map_err(err)?;
            let event: EntityEvent =
                serde_json::from_str(&payload).map_err(|e| PortError(e.to_string()))?;
            out.push(StoredEntityEvent {
                seq: seq as u64,
                event,
            });
        }
        Ok(out)
    }

    async fn snapshot(&self) -> PortResult<EntitySnapshot> {
        let events = self
            .events()
            .await?
            .into_iter()
            .map(|stored| stored.event)
            .collect::<Vec<_>>();
        Ok(project(&events))
    }

    fn export_jsonl(&self) -> PortResult<String> {
        let conn = self.conn.lock().map_err(lock)?;
        let mut stmt = conn
            .prepare("SELECT seq, payload_json FROM entity_event ORDER BY seq")
            .map_err(err)?;
        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(err)?;
        let mut out = String::new();
        for row in rows {
            let (seq, payload) = row.map_err(err)?;
            out.push_str(&format!("{{\"seq\":{seq},\"event\":{payload}}}\n"));
        }
        Ok(out)
    }
}

pub fn import_jsonl(path: &Path, jsonl: &str) -> PortResult<()> {
    let conn = Connection::open(path).map_err(err)?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;")
        .map_err(err)?;
    migrate(&conn)?;
    conn.execute("DELETE FROM entity_event", []).map_err(err)?;
    for line in jsonl.lines().filter(|line| !line.trim().is_empty()) {
        let value: serde_json::Value =
            serde_json::from_str(line).map_err(|e| PortError(e.to_string()))?;
        let seq = value["seq"]
            .as_u64()
            .ok_or_else(|| PortError("import line has no seq".into()))?;
        let event: EntityEvent =
            serde_json::from_value(value["event"].clone()).map_err(|e| PortError(e.to_string()))?;
        let payload = serde_json::to_string(&event).map_err(|e| PortError(e.to_string()))?;
        conn.execute(
            "INSERT INTO entity_event (seq, event_id, kind, payload_json, created_at)
             VALUES (?1, ?2, ?3, ?4, unixepoch())",
            params![seq as i64, new_id(), event.kind(), payload],
        )
        .map_err(err)?;
    }
    Ok(())
}

pub struct SqliteEntityFactory {
    db: PathBuf,
}

impl SqliteEntityFactory {
    pub fn new(db: impl Into<PathBuf>) -> Self {
        Self { db: db.into() }
    }
}

impl EntityFactory for SqliteEntityFactory {
    fn open(&self) -> PortResult<EntityRefs> {
        let store = Arc::new(SqliteEntityStore::open(&self.db)?);
        Ok(EntityRefs {
            id: store.entity_id.clone(),
            append: store.clone(),
            read: store.clone(),
        })
    }
}

fn migrate(conn: &Connection) -> PortResult<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS entity_event (
            seq INTEGER PRIMARY KEY AUTOINCREMENT,
            event_id TEXT NOT NULL,
            kind TEXT NOT NULL,
            payload_json TEXT NOT NULL,
            created_at INTEGER NOT NULL
         );",
    )
    .map_err(err)
}

fn insert_event(conn: &Connection, event: &EntityEvent) -> PortResult<u64> {
    let event = redact_entity_event(event);
    let payload = serde_json::to_string(&event).map_err(|e| PortError(e.to_string()))?;
    conn.execute(
        "INSERT INTO entity_event (event_id, kind, payload_json, created_at)
         VALUES (?1, ?2, ?3, unixepoch())",
        params![new_id(), event.kind(), payload],
    )
    .map_err(err)?;
    Ok(conn.last_insert_rowid() as u64)
}

fn new_id() -> String {
    ulid::Ulid::new().to_string()
}

fn err(error: rusqlite::Error) -> PortError {
    PortError(error.to_string())
}

fn lock<T>(_: T) -> PortError {
    PortError("entity store lock poisoned".into())
}
