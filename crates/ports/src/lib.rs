//! Ports declared by the consumer side. No concrete adapter is named here.

use async_trait::async_trait;
use orexis_core::{Action, EventRef, Message, Observation, SessionEvent, StoredEvent, ToolSpec};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortError(pub String);

impl std::fmt::Display for PortError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for PortError {}

pub type PortResult<T> = Result<T, PortError>;

#[async_trait]
pub trait Provider: Send + Sync {
    async fn complete(&self, messages: &[Message], tools: &[ToolSpec], step: usize) -> Vec<Action>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decoded {
    pub actions: Vec<Action>,
    pub errors: Vec<String>,
}

pub trait Codec: Send + Sync {
    fn name(&self) -> &'static str;
    fn prompt_fragment(&self, tools: &[ToolSpec]) -> String;
    fn decode(&self, output: &str, tools: &[ToolSpec]) -> Decoded;
    fn uses_native_tools(&self) -> bool {
        false
    }
    fn grammar(&self, _tools: &[ToolSpec]) -> Option<String> {
        None
    }
}

#[async_trait]
pub trait ToolExecutor: Send + Sync {
    async fn execute(&self, name: &str, args: &str) -> Observation;
}

#[async_trait]
pub trait SessionAppend: Send + Sync {
    async fn append(&self, event: SessionEvent) -> PortResult<EventRef>;
}

#[async_trait]
pub trait SessionRead: Send + Sync {
    async fn events(&self) -> PortResult<Vec<StoredEvent>>;
}

#[async_trait]
pub trait SessionFork: Send + Sync {
    async fn fork(&self, at_seq: u64) -> PortResult<String>;
}

pub trait EventSink: Send + Sync {
    fn emit(&self, event: &SessionEvent);
}
