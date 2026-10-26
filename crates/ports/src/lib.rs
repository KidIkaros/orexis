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
pub trait DeltaSink: Send + Sync {
    fn delta(&self, text: &str);
}

pub struct NullDeltaSink;

impl DeltaSink for NullDeltaSink {
    fn delta(&self, _text: &str) {}
}

#[async_trait]
pub trait Provider: Send + Sync {
    async fn complete(
        &self,
        messages: &[Message],
        tools: &[ToolSpec],
        step: usize,
    ) -> PortResult<ProviderResponse>;

    /// Streaming variant. The default emits the final text as one delta.
    async fn stream(
        &self,
        messages: &[Message],
        tools: &[ToolSpec],
        step: usize,
        sink: &dyn DeltaSink,
    ) -> PortResult<ProviderResponse> {
        let response = self.complete(messages, tools, step).await?;
        if let Some(Action::Say { text }) = response
            .actions
            .iter()
            .find(|action| matches!(action, Action::Say { .. }))
        {
            sink.delta(text);
        }
        Ok(response)
    }
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub path: String,
    pub line: u32,
    pub column: u32,
    pub severity: String,
    pub message: String,
}

#[async_trait]
pub trait Lsp: Send + Sync {
    async fn diagnostics(&self, path: &str, content: &str) -> PortResult<Vec<Diagnostic>>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifyResult {
    pub command: String,
    pub passed: bool,
    pub output: String,
}

#[async_trait]
pub trait Verifier: Send + Sync {
    async fn verify(&self) -> PortResult<VerifyResult>;
}

pub trait Checkpointer: Send + Sync {
    fn create(&self, label: &str) -> PortResult<String>;
    fn restore(&self, id: &str) -> PortResult<()>;
    fn diff(&self, id: &str) -> PortResult<String>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilePreview {
    pub path: String,
    pub content: String,
    pub truncated: bool,
}

pub trait Workspace: Send + Sync {
    fn list(&self, path: &str) -> PortResult<Vec<FileEntry>>;
    fn read(&self, path: &str, max_bytes: usize) -> PortResult<FilePreview>;
    fn changed(&self) -> PortResult<Vec<String>>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillTier {
    Trusted,
    Verified,
    Community,
    Quarantined,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillStatus {
    Allowed,
    CommunityBlocked,
    Quarantined,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillSummary {
    pub name: String,
    pub version: String,
    pub tier: SkillTier,
    pub status: SkillStatus,
    pub sha256: Option<String>,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillContent {
    pub name: String,
    pub text: String,
}

pub trait Skills: Send + Sync {
    fn list(&self) -> Vec<SkillSummary>;
    fn load(&self, name: &str) -> PortResult<SkillContent>;
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

/// Session-scoped approval overrides: tools the user chose to always allow.
pub trait ApprovalOverrides: Send + Sync {
    fn allows(&self, session: &str, tool: &str) -> bool;
    fn remember(&self, session: &str, tool: &str);
    fn clear(&self, session: &str);
}

pub trait EventSink: Send + Sync {
    fn emit(&self, event: &SessionEvent, seq: u64);
}
