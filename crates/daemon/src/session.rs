//! Session registry and the broadcast sink.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use orexis_core::SessionEvent;
use orexis_ports::{
    DeltaSink, EventSink, PortError, PortResult, SessionAppend, SessionFactory, SessionRead,
    SessionRefs,
};
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

pub struct BroadcastDeltaSink {
    tx: broadcast::Sender<ServerMsg>,
}

impl BroadcastDeltaSink {
    pub fn new(tx: broadcast::Sender<ServerMsg>) -> Self {
        Self { tx }
    }
}

impl DeltaSink for BroadcastDeltaSink {
    fn delta(&self, text: &str) {
        let _ = self.tx.send(ServerMsg::Delta {
            text: text.to_string(),
        });
    }
}

pub struct SessionRuntime {
    pub id: String,
    pub append: Arc<dyn SessionAppend>,
    pub read: Arc<dyn SessionRead>,
    pub tx: broadcast::Sender<ServerMsg>,
    pub turn: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl SessionRuntime {
    fn new(refs: SessionRefs) -> Self {
        let (tx, _) = broadcast::channel(1024);
        Self {
            id: refs.id,
            append: refs.append,
            read: refs.read,
            tx,
            turn: Mutex::new(None),
        }
    }
}

pub struct SessionRegistry {
    factory: Arc<dyn SessionFactory>,
    sessions: Mutex<HashMap<String, Arc<SessionRuntime>>>,
}

impl SessionRegistry {
    pub fn new(factory: Arc<dyn SessionFactory>) -> Self {
        Self {
            factory,
            sessions: Mutex::new(HashMap::new()),
        }
    }

    pub fn get_or_create(&self, session: Option<&str>) -> PortResult<Arc<SessionRuntime>> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| PortError("registry lock poisoned".into()))?;
        if let Some(id) = session {
            if let Some(runtime) = sessions.get(id) {
                return Ok(runtime.clone());
            }
        }
        let refs = self.factory.open(session)?;
        let runtime = Arc::new(SessionRuntime::new(refs));
        sessions.insert(runtime.id.clone(), runtime.clone());
        Ok(runtime)
    }

    pub fn list(&self) -> Vec<SessionInfo> {
        let mut ids = self.factory.list().unwrap_or_default();
        let running: HashMap<String, bool> = self
            .sessions
            .lock()
            .map(|sessions| {
                sessions
                    .iter()
                    .map(|(id, runtime)| {
                        let active = runtime
                            .turn
                            .lock()
                            .map(|turn| turn.is_some())
                            .unwrap_or(false);
                        (id.clone(), active)
                    })
                    .collect()
            })
            .unwrap_or_default();
        ids.extend(running.keys().cloned());
        ids.sort();
        ids.dedup();
        ids.into_iter()
            .map(|id| SessionInfo {
                status: if running.get(&id).copied().unwrap_or(false) {
                    "running".to_string()
                } else {
                    "idle".to_string()
                },
                id,
            })
            .collect()
    }
}
