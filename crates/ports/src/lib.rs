//! Ports declared by the consumer side. No concrete adapter is named here.

use async_trait::async_trait;
use orexis_core::{Action, Message, Observation, SessionEvent};

#[async_trait]
pub trait Provider: Send + Sync {
    async fn complete(&self, messages: &[Message], step: usize) -> Vec<Action>;
}

#[async_trait]
pub trait ToolExecutor: Send + Sync {
    async fn execute(&self, name: &str, args: &str) -> Observation;
}

#[async_trait]
pub trait SessionAppend: Send + Sync {
    async fn append(&self, event: SessionEvent);
}

#[async_trait]
pub trait SessionRead: Send + Sync {
    async fn events(&self) -> Vec<SessionEvent>;
}

pub trait EventSink: Send + Sync {
    fn emit(&self, event: &SessionEvent);
}
