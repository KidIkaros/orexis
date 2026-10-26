//! The daemon's approval hub: a tool gate that asks connected clients.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use orexis_ports::{ApprovalGate, ApprovalOverrides, PortError, PortResult};
use tokio::sync::{broadcast, oneshot};

use crate::proto::ServerMsg;

/// In-memory, session-scoped overrides.
#[derive(Default)]
pub struct MemoryOverrides {
    values: Mutex<HashMap<String, HashSet<String>>>,
}

impl ApprovalOverrides for MemoryOverrides {
    fn allows(&self, session: &str, tool: &str) -> bool {
        self.values
            .lock()
            .map(|values| {
                values
                    .get(session)
                    .is_some_and(|tools| tools.contains(tool))
            })
            .unwrap_or(false)
    }

    fn remember(&self, session: &str, tool: &str) {
        if let Ok(mut values) = self.values.lock() {
            values
                .entry(session.to_string())
                .or_default()
                .insert(tool.to_string());
        }
    }

    fn clear(&self, session: &str) {
        if let Ok(mut values) = self.values.lock() {
            values.remove(session);
        }
    }
}

pub struct ApprovalHub {
    next: AtomicU64,
    pending: Mutex<HashMap<u64, (String, String, oneshot::Sender<bool>)>>,
    overrides: Arc<dyn ApprovalOverrides>,
    tx: broadcast::Sender<ServerMsg>,
}

impl ApprovalHub {
    pub fn new(overrides: Arc<dyn ApprovalOverrides>) -> Self {
        let (tx, _) = broadcast::channel(64);
        Self {
            next: AtomicU64::new(0),
            pending: Mutex::new(HashMap::new()),
            overrides,
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

    pub fn clear_session(&self, session: &str) {
        self.overrides.clear(session);
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
            if let Some((session, tool, sender)) = pending.remove(&id) {
                if yes && always {
                    self.overrides.remember(&session, &tool);
                }
                let _ = sender.send(yes);
            }
        }
    }
}

#[derive(Clone)]
pub struct HubGate {
    hub: Arc<ApprovalHub>,
    session: String,
    timeout: Duration,
}

impl HubGate {
    pub fn new(hub: Arc<ApprovalHub>, session: impl Into<String>) -> Self {
        Self {
            hub,
            session: session.into(),
            timeout: Duration::from_secs(300),
        }
    }

    #[cfg(test)]
    pub fn with_timeout(
        hub: Arc<ApprovalHub>,
        session: impl Into<String>,
        timeout: Duration,
    ) -> Self {
        Self {
            hub,
            session: session.into(),
            timeout,
        }
    }
}

#[async_trait]
impl ApprovalGate for HubGate {
    async fn approve(&self, tool: &str, args: &str) -> PortResult<bool> {
        if self.hub.overrides.allows(&self.session, tool) {
            return Ok(true);
        }
        let id = self.hub.next.fetch_add(1, Ordering::SeqCst) + 1;
        let (sender, receiver) = oneshot::channel();
        self.hub
            .pending
            .lock()
            .map_err(|_| PortError("approval lock poisoned".into()))?
            .insert(id, (self.session.clone(), tool.to_string(), sender));
        let _ = self.hub.tx.send(ServerMsg::ApprovalRequest {
            id,
            session_id: self.session.clone(),
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

    fn hub() -> Arc<ApprovalHub> {
        Arc::new(ApprovalHub::new(Arc::new(MemoryOverrides::default())))
    }

    #[tokio::test]
    async fn timeout_removes_the_pending_sender() {
        let hub = hub();
        let gate = HubGate::with_timeout(hub.clone(), "s1", Duration::from_millis(20));
        let error = gate.approve("shell", "{}").await.expect_err("timeout");
        assert!(error.to_string().contains("timed out"));
        assert_eq!(hub.pending_len(), 0);
    }

    #[tokio::test]
    async fn always_allows_the_same_tool_in_the_same_session() {
        let hub = hub();
        let gate = HubGate::new(hub.clone(), "s1");
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

    #[tokio::test]
    async fn always_does_not_leak_across_sessions() {
        let hub = hub();
        let first = HubGate::new(hub.clone(), "s1");
        let mut events = hub.subscribe();
        let task = tokio::spawn({
            let gate = first.clone();
            async move { gate.approve("shell", "echo").await }
        });
        let id = match events.recv().await.expect("request") {
            ServerMsg::ApprovalRequest { id, .. } => id,
            other => panic!("unexpected {other:?}"),
        };
        hub.resolve(id, true, true);
        assert!(task.await.expect("task").expect("approved"));

        let other = HubGate::new(hub.clone(), "s2");
        let task = tokio::spawn(async move { other.approve("shell", "echo").await });
        let id = match events.recv().await.expect("second request") {
            ServerMsg::ApprovalRequest { id, .. } => id,
            other => panic!("unexpected {other:?}"),
        };
        hub.resolve(id, false, false);
        assert!(!task.await.expect("task").expect("denied"));
    }

    #[tokio::test]
    async fn clear_session_forgets_overrides() {
        let hub = hub();
        let gate = HubGate::new(hub.clone(), "s1");
        let mut events = hub.subscribe();
        let task = tokio::spawn({
            let gate = gate.clone();
            async move { gate.approve("shell", "echo").await }
        });
        let id = match events.recv().await.expect("request") {
            ServerMsg::ApprovalRequest { id, .. } => id,
            other => panic!("unexpected {other:?}"),
        };
        hub.resolve(id, true, true);
        assert!(task.await.expect("task").expect("approved"));

        hub.clear_session("s1");
        let task = tokio::spawn(async move { gate.approve("shell", "echo").await });
        let id = match events.recv().await.expect("request after clear") {
            ServerMsg::ApprovalRequest { id, .. } => id,
            other => panic!("unexpected {other:?}"),
        };
        hub.resolve(id, false, false);
        assert!(!task.await.expect("task").expect("denied"));
    }
}
