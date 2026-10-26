//! Ports declared by the consumer side. No concrete adapter is named here.

use async_trait::async_trait;
use orexis_core::{
    Action, ContextFile, EventRef, Message, Observation, ProviderResponse, SessionEvent,
    StoredEvent, ToolSpec,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortError(pub String);

impl std::fmt::Display for PortError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for PortError {}

pub type PortResult<T> = Result<T, PortError>;

#[derive(Clone)]
pub struct SessionRefs {
    pub id: String,
    pub append: std::sync::Arc<dyn SessionAppend>,
    pub read: std::sync::Arc<dyn SessionRead>,
}

pub trait SessionFactory: Send + Sync {
    fn open(&self, session: Option<&str>) -> PortResult<SessionRefs>;
    fn list(&self) -> PortResult<Vec<String>>;
}

#[async_trait]
pub trait Provider: Send + Sync {
    async fn complete(
        &self,
        messages: &[Message],
        tools: &[ToolSpec],
        step: usize,
    ) -> PortResult<ProviderResponse>;
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
    async fn execute(&self, name: &str, args: &str) -> PortResult<Observation>;
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

pub trait ContextSource: Send + Sync {
    fn load(&self) -> Vec<ContextFile>;
}

pub trait Summarizer: Send + Sync {
    fn summarize(&self, events: &[SessionEvent]) -> String;
}

#[async_trait]
pub trait ApprovalGate: Send + Sync {
    async fn approve(&self, tool: &str, args: &str) -> PortResult<bool>;
}

pub trait EventSink: Send + Sync {
    fn emit(&self, event: &SessionEvent, seq: u64);
}
