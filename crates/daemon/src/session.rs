//! Session registry and the broadcast sink.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use orexis_adapters_session_sqlite::SqliteSessionStore;
use orexis_core::SessionEvent;
use orexis_ports::{EventSink, PortResult};
use tokio::sync::broadcast;

use crate::proto::{ServerMsg, SessionInfo};

pub struct BroadcastSink {
    tx: broadcast::Sender<ServerMsg>,
}

impl BroadcastSink {
    pub fn new(tx: broadcast::Sender<ServerMsg>) -> Self {
        Self { tx }
    }
}

impl EventSink for BroadcastSink {
    fn emit(&self, event: &SessionEvent, seq: u64) {
        let _ = self.tx.send(ServerMsg::Event {
            seq,
            replay: false,
            event: event.clone(),
        });
    }
}

pub struct SessionRuntime {
    pub id: String,
    pub store: Arc<SqliteSessionStore>,
    pub tx: broadcast::Sender<ServerMsg>,
    pub turn: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl SessionRuntime {
    fn new(store: Arc<SqliteSessionStore>) -> Self {
        let (tx, _) = broadcast::channel(1024);
        Self {
            id: store.session_id().to_string(),
            store,
            tx,
            turn: Mutex::new(None),
        }
    }
}

pub struct SessionRegistry {
    db: PathBuf,
    sessions: Mutex<HashMap<String, Arc<SessionRuntime>>>,
}

impl SessionRegistry {
    pub fn new(db: PathBuf) -> Self {
        Self {
            db,
            sessions: Mutex::new(HashMap::new()),
        }
    }

    pub fn get_or_create(&self, session: Option<&str>) -> PortResult<Arc<SessionRuntime>> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| orexis_ports::PortError("registry lock poisoned".into()))?;
        if let Some(id) = session {
            if let Some(runtime) = sessions.get(id) {
                return Ok(runtime.clone());
            }
            let store = Arc::new(SqliteSessionStore::open_session(&self.db, id)?);
            let runtime = Arc::new(SessionRuntime::new(store));
            sessions.insert(id.to_string(), runtime.clone());
            return Ok(runtime);
        }
        let store = Arc::new(SqliteSessionStore::open(&self.db)?);
        let runtime = Arc::new(SessionRuntime::new(store));
        sessions.insert(runtime.id.clone(), runtime.clone());
        Ok(runtime)
    }

    pub fn list(&self) -> Vec<SessionInfo> {
        self.sessions
            .lock()
            .map(|sessions| {
                sessions
                    .values()
                    .map(|runtime| SessionInfo {
                        id: runtime.id.clone(),
                        status: if runtime
                            .turn
                            .lock()
                            .map(|turn| turn.is_some())
                            .unwrap_or(false)
                        {
                            "running".to_string()
                        } else {
                            "idle".to_string()
                        },
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}
