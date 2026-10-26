//! The daemon's approval hub: a tool gate that asks connected clients.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use orexis_ports::{ApprovalGate, PortError, PortResult};
use tokio::sync::{broadcast, oneshot};

use crate::proto::ServerMsg;

pub struct ApprovalHub {
    next: AtomicU64,
    pending: Mutex<HashMap<u64, (String, oneshot::Sender<bool>)>>,
    always: Mutex<HashSet<String>>,
    tx: broadcast::Sender<ServerMsg>,
}

impl ApprovalHub {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(64);
        Self {
            next: AtomicU64::new(0),
            pending: Mutex::new(HashMap::new()),
            always: Mutex::new(HashSet::new()),
            tx,
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ServerMsg> {
        self.tx.subscribe()
    }

    pub fn remove(&self, id: u64) {
        if let Ok(mut pending) = self.pending.lock() {
            pending.remove(&id);
        }
    }

    #[cfg(test)]
    pub fn pending_len(&self) -> usize {
        self.pending
            .lock()
            .map(|pending| pending.len())
            .unwrap_or(0)
    }

    pub fn resolve(&self, id: u64, yes: bool, always: bool) {
        if let Ok(mut pending) = self.pending.lock() {
            if let Some((tool, sender)) = pending.remove(&id) {
                if yes && always {
                    if let Ok(mut allowed) = self.always.lock() {
                        allowed.insert(tool);
                    }
                }
                let _ = sender.send(yes);
            }
        }
    }
}

#[derive(Clone)]
pub struct HubGate {
    hub: Arc<ApprovalHub>,
    timeout: Duration,
}

impl HubGate {
    pub fn new(hub: Arc<ApprovalHub>) -> Self {
        Self {
            hub,
            timeout: Duration::from_secs(300),
        }
    }

    #[cfg(test)]
    pub fn with_timeout(hub: Arc<ApprovalHub>, timeout: Duration) -> Self {
        Self { hub, timeout }
    }
}

#[async_trait]
impl ApprovalGate for HubGate {
    async fn approve(&self, tool: &str, args: &str) -> PortResult<bool> {
        if self
            .hub
            .always
            .lock()
            .map(|allowed| allowed.contains(tool))
            .unwrap_or(false)
        {
            return Ok(true);
        }
        let id = self.hub.next.fetch_add(1, Ordering::SeqCst) + 1;
        let (sender, receiver) = oneshot::channel();
        self.hub
            .pending
            .lock()
            .map_err(|_| PortError("approval lock poisoned".into()))?
            .insert(id, (tool.to_string(), sender));
        let _ = self.hub.tx.send(ServerMsg::ApprovalRequest {
            id,
            tool: tool.to_string(),
            args: args.to_string(),
        });
        match tokio::time::timeout(self.timeout, receiver).await {
            Ok(Ok(yes)) => Ok(yes),
            Ok(Err(_)) => {
                self.hub.remove(id);
                Err(PortError("approval channel closed".into()))
            }
            Err(_) => {
                self.hub.remove(id);
                Err(PortError("approval timed out".into()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn timeout_removes_the_pending_sender() {
        let hub = Arc::new(ApprovalHub::new());
        let gate = HubGate::with_timeout(hub.clone(), Duration::from_millis(20));
        let error = gate.approve("shell", "{}").await.expect_err("timeout");
        assert!(error.to_string().contains("timed out"));
        assert_eq!(hub.pending_len(), 0);
    }

    #[tokio::test]
    async fn always_allows_the_same_tool() {
        let hub = Arc::new(ApprovalHub::new());
        let gate = HubGate::new(hub.clone());
        let mut events = hub.subscribe();
        let first = tokio::spawn({
            let gate = gate.clone();
            async move { gate.approve("shell", "echo one").await }
        });
        let id = match events.recv().await.expect("request") {
            ServerMsg::ApprovalRequest { id, .. } => id,
            other => panic!("unexpected {other:?}"),
        };
        hub.resolve(id, true, true);
        assert!(first.await.expect("task").expect("approve"));

        assert!(gate.approve("shell", "echo two").await.expect("always"));
        assert!(events.try_recv().is_err(), "no second request expected");
    }
}
