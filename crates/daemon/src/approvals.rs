//! The daemon's approval hub: a tool gate that asks connected clients.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use orexis_ports::{ApprovalGate, PortError, PortResult};
use tokio::sync::{broadcast, oneshot};

use crate::proto::ServerMsg;

pub struct ApprovalHub {
    next: AtomicU64,
    pending: Mutex<HashMap<u64, oneshot::Sender<bool>>>,
    tx: broadcast::Sender<ServerMsg>,
}

impl ApprovalHub {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(64);
        Self {
            next: AtomicU64::new(0),
            pending: Mutex::new(HashMap::new()),
            tx,
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ServerMsg> {
        self.tx.subscribe()
    }

    pub fn resolve(&self, id: u64, yes: bool) {
        if let Ok(mut pending) = self.pending.lock() {
            if let Some(sender) = pending.remove(&id) {
                let _ = sender.send(yes);
            }
        }
    }
}

pub struct HubGate {
    hub: Arc<ApprovalHub>,
}

impl HubGate {
    pub fn new(hub: Arc<ApprovalHub>) -> Self {
        Self { hub }
    }
}

#[async_trait]
impl ApprovalGate for HubGate {
    async fn approve(&self, tool: &str, args: &str) -> PortResult<bool> {
        let id = self.hub.next.fetch_add(1, Ordering::SeqCst) + 1;
        let (sender, receiver) = oneshot::channel();
        self.hub
            .pending
            .lock()
            .map_err(|_| PortError("approval lock poisoned".into()))?
            .insert(id, sender);
        let _ = self.hub.tx.send(ServerMsg::ApprovalRequest {
            id,
            tool: tool.to_string(),
            args: args.to_string(),
        });
        match tokio::time::timeout(Duration::from_secs(300), receiver).await {
            Ok(Ok(yes)) => Ok(yes),
            Ok(Err(_)) => Err(PortError("approval channel closed".into())),
            Err(_) => Err(PortError("approval timed out".into())),
        }
    }
}
