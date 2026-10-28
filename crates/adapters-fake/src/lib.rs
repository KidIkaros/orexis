//! Deterministic test adapters for the vertical slice. No I/O.

use std::sync::Mutex;
use std::time::Duration;

use async_trait::async_trait;
use std::collections::BTreeMap;

use orexis_core::{
    Action, EventRef, Message, Observation, ProviderResponse, SessionEvent, StoredEvent, Thinking,
    ToolSpec,
};
use orexis_ports::{
    EventSink, PortError, PortResult, Provider, SessionAppend, SessionRead, ToolExecutor,
};

pub struct ScriptedProvider {
    script: Vec<Vec<Action>>,
}

impl ScriptedProvider {
    pub fn new(script: Vec<Vec<Action>>) -> Self {
        Self { script }
    }
}

pub struct SlowProvider {
    delay: Duration,
}

impl SlowProvider {
    pub fn new(delay: Duration) -> Self {
        Self { delay }
    }
}

#[async_trait]
impl Provider for SlowProvider {
    async fn complete(
        &self,
        _messages: &[Message],
        _tools: &[ToolSpec],
        _step: usize,
        _thinking: Thinking,
    ) -> PortResult<ProviderResponse> {
        tokio::time::sleep(self.delay).await;
        Ok(ProviderResponse {
            actions: vec![Action::Done {
                summary: "slow".to_string(),
            }],
            raw_text: "slow".to_string(),
            usage: None,
        })
    }
}

#[async_trait]
impl Provider for ScriptedProvider {
    async fn complete(
        &self,
        _messages: &[Message],
        _tools: &[ToolSpec],
        step: usize,
        _thinking: Thinking,
    ) -> PortResult<ProviderResponse> {
        let actions = self.script.get(step).cloned().unwrap_or_else(|| {
            vec![Action::Done {
                summary: "script exhausted".to_string(),
            }]
        });
        Ok(ProviderResponse {
            actions,
            raw_text: String::new(),
            // Scripted usage keeps the bench's token accounting honest
            // in dry runs: one prompt token per step, none generated.
            usage: Some(orexis_core::TokenUsage {
                prompt: step as u64 + 1,
                completion: 0,
            }),
        })
    }
}

pub struct EchoTools;

#[async_trait]
impl ToolExecutor for EchoTools {
    async fn execute(&self, name: &str, args: &str) -> PortResult<Observation> {
        Ok(Observation {
            name: name.to_string(),
            output: format!("ok({args})"),
        })
    }
}

#[derive(Default)]
pub struct InMemorySession {
    events: Mutex<Vec<StoredEvent>>,
}

#[async_trait]
impl SessionAppend for InMemorySession {
    async fn append(&self, event: SessionEvent) -> PortResult<EventRef> {
        let mut events = self
            .events
            .lock()
            .map_err(|_| PortError("session lock poisoned".into()))?;
        let seq = events.len() as u64 + 1;
        let event_id = format!("evt-{seq:04}");
        events.push(StoredEvent {
            seq,
            event_id: event_id.clone(),
            generation_id: 1,
            event,
        });
        Ok(EventRef { seq, event_id })
    }
}

#[async_trait]
impl SessionRead for InMemorySession {
    async fn events(&self) -> PortResult<Vec<StoredEvent>> {
        self.events
            .lock()
            .map(|events| events.clone())
            .map_err(|_| PortError("session lock poisoned".into()))
    }
}

#[derive(Default)]
pub struct RecordingSink {
    lines: Mutex<Vec<String>>,
}

impl RecordingSink {
    pub fn lines(&self) -> Vec<String> {
        self.lines
            .lock()
            .map(|lines| lines.clone())
            .unwrap_or_default()
    }
}

impl EventSink for RecordingSink {
    fn emit(&self, event: &SessionEvent, seq: u64) {
        if let Ok(mut lines) = self.lines.lock() {
            lines.push(format!("{seq}: {event:?}"));
        }
    }
}

/// The slice's plain-text projection. The real TUI adapter replaces this in M1.
pub fn render_transcript(events: &[SessionEvent]) -> String {
    let mut out = String::new();
    for event in events {
        match event {
            SessionEvent::UserMessage { text } => out.push_str(&format!("you › {text}\n")),
            SessionEvent::ModelResponse { text, .. } if !text.is_empty() => {
                out.push_str(&format!("orexis › {text}\n"))
            }
            SessionEvent::ToolCall { name, args, .. } => {
                out.push_str(&format!("  ▸ {name} {args}\n"))
            }
            SessionEvent::Observation { name, output, .. } => {
                out.push_str(&format!("    {name} → {output}\n"))
            }
            SessionEvent::Done { summary } => out.push_str(&format!("✓ {summary}\n")),
            _ => {}
        }
    }
    out
}

// --- Team fakes (M8-01, ADR-0020) ---

/// The board in memory. The contracts the sqlite adapter must keep are
/// encoded against this fake: edges gate readiness, caps refuse visibly,
/// reopen preserves.
pub struct FakeBoard {
    tasks: std::sync::Mutex<BTreeMap<String, orexis_core::BoardTask>>,
    next_seq: std::sync::Mutex<u64>,
    cap: usize,
}

impl FakeBoard {
    pub fn new(cap: usize) -> Self {
        Self {
            tasks: std::sync::Mutex::new(BTreeMap::new()),
            next_seq: std::sync::Mutex::new(1),
            cap,
        }
    }
}

#[async_trait]
impl orexis_ports::BoardStore for FakeBoard {
    async fn create(
        &self,
        title: &str,
        required_strategy: &str,
        depends_on: Vec<String>,
    ) -> PortResult<orexis_core::BoardTask> {
        let mut tasks = self.tasks.lock().expect("tasks");
        if tasks.len() >= self.cap {
            return Err(PortError(format!("the board is full ({})", self.cap)));
        }
        for edge in &depends_on {
            if !tasks.contains_key(edge) {
                return Err(PortError(format!("unknown task in depends_on: {edge}")));
            }
        }
        let mut seq = self.next_seq.lock().expect("seq");
        let id = format!("task-{}", *seq);
        *seq += 1;
        let task = orexis_core::BoardTask {
            id: id.clone(),
            title: title.to_string(),
            required_strategy: required_strategy.to_string(),
            owner: None,
            status: orexis_core::TaskStatus::Queued,
            depends_on,
            seq: *seq,
        };
        tasks.insert(id, task.clone());
        Ok(task)
    }

    async fn get(&self, id: &str) -> PortResult<Option<orexis_core::BoardTask>> {
        Ok(self.tasks.lock().expect("tasks").get(id).cloned())
    }

    async fn list(&self) -> PortResult<Vec<orexis_core::BoardTask>> {
        Ok(self
            .tasks
            .lock()
            .expect("tasks")
            .values()
            .cloned()
            .collect())
    }

    async fn ready(&self) -> PortResult<Vec<orexis_core::BoardTask>> {
        let tasks = self.tasks.lock().expect("tasks");
        Ok(tasks
            .values()
            .filter(|task| {
                task.status == orexis_core::TaskStatus::Queued
                    && task.depends_on.iter().all(|edge| {
                        tasks
                            .get(edge)
                            .is_some_and(|dep| dep.status == orexis_core::TaskStatus::Done)
                    })
            })
            .cloned()
            .collect())
    }

    async fn admit(&self, id: &str) -> PortResult<orexis_core::BoardTask> {
        let mut tasks = self.tasks.lock().expect("tasks");
        let task = tasks
            .get_mut(id)
            .ok_or_else(|| PortError(format!("no task {id}")))?;
        if task.status != orexis_core::TaskStatus::Queued {
            return Err(PortError(format!("task {id} is not queued")));
        }
        task.status = orexis_core::TaskStatus::Admitted;
        Ok(task.clone())
    }

    async fn start(&self, id: &str, owner: &str) -> PortResult<orexis_core::BoardTask> {
        let mut tasks = self.tasks.lock().expect("tasks");
        let task = tasks
            .get_mut(id)
            .ok_or_else(|| PortError(format!("no task {id}")))?;
        if task.status != orexis_core::TaskStatus::Admitted {
            return Err(PortError(format!("task {id} is not admitted")));
        }
        task.status = orexis_core::TaskStatus::Running;
        task.owner = Some(owner.to_string());
        Ok(task.clone())
    }

    async fn complete(&self, id: &str) -> PortResult<orexis_core::BoardTask> {
        let mut tasks = self.tasks.lock().expect("tasks");
        let task = tasks
            .get_mut(id)
            .ok_or_else(|| PortError(format!("no task {id}")))?;
        if task.status != orexis_core::TaskStatus::Running {
            return Err(PortError(format!("task {id} is not running")));
        }
        task.status = orexis_core::TaskStatus::Done;
        Ok(task.clone())
    }

    async fn fail(&self, id: &str, reason: &str) -> PortResult<orexis_core::BoardTask> {
        let mut tasks = self.tasks.lock().expect("tasks");
        let task = tasks
            .get_mut(id)
            .ok_or_else(|| PortError(format!("no task {id}")))?;
        task.status = orexis_core::TaskStatus::Failed;
        task.title = format!("{} — failed: {reason}", task.title);
        Ok(task.clone())
    }
}

/// The bus in memory.
#[derive(Default)]
pub struct FakeBus {
    messages: std::sync::Mutex<Vec<orexis_core::BusMessage>>,
}

impl FakeBus {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl orexis_ports::MessageStore for FakeBus {
    async fn post(
        &self,
        from: &str,
        to: Option<&str>,
        text: &str,
    ) -> PortResult<orexis_core::BusMessage> {
        let mut messages = self.messages.lock().expect("messages");
        let seq = messages.last().map(|message| message.seq + 1).unwrap_or(1);
        let message = orexis_core::BusMessage {
            seq,
            from: from.to_string(),
            to: to.map(str::to_string),
            text: text.chars().take(64 * 1024).collect(),
        };
        messages.push(message.clone());
        Ok(message)
    }

    async fn read_since(
        &self,
        seq: u64,
        for_worker: Option<&str>,
    ) -> PortResult<Vec<orexis_core::BusMessage>> {
        let messages = self.messages.lock().expect("messages");
        Ok(messages
            .iter()
            .filter(|message| {
                message.seq > seq
                    && match for_worker {
                        None => true,
                        Some(worker) => {
                            message.to.is_none() || message.to.as_deref() == Some(worker)
                        }
                    }
            })
            .cloned()
            .collect())
    }
}

/// The blackboard in memory: artifacts and leases with wall-clock expiry.
pub struct FakeBlackboard {
    artifacts: std::sync::Mutex<BTreeMap<String, Vec<u8>>>,
    leases: std::sync::Mutex<BTreeMap<String, orexis_core::Lease>>,
}

impl FakeBlackboard {
    pub fn new() -> Self {
        Self {
            artifacts: std::sync::Mutex::new(BTreeMap::new()),
            leases: std::sync::Mutex::new(BTreeMap::new()),
        }
    }
}

impl Default for FakeBlackboard {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl orexis_ports::ArtifactStore for FakeBlackboard {
    async fn put(&self, name: &str, bytes: &[u8]) -> PortResult<()> {
        if name.is_empty() || name.contains("..") || name.starts_with('/') {
            return Err(PortError(format!("bad artifact name: {name}")));
        }
        let mut artifacts = self.artifacts.lock().expect("artifacts");
        artifacts.insert(name.to_string(), bytes.to_vec());
        Ok(())
    }

    async fn get(&self, name: &str) -> PortResult<Option<Vec<u8>>> {
        Ok(self.artifacts.lock().expect("artifacts").get(name).cloned())
    }

    async fn list(&self) -> PortResult<Vec<String>> {
        Ok(self
            .artifacts
            .lock()
            .expect("artifacts")
            .keys()
            .cloned()
            .collect())
    }

    async fn lease(
        &self,
        path: &str,
        owner: &str,
        ttl_secs: u64,
    ) -> PortResult<orexis_core::Lease> {
        let mut leases = self.leases.lock().expect("leases");
        if let Some(existing) = leases.get(path) {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            if existing.expires_at > now && existing.owner != owner {
                return Err(PortError(format!(
                    "{} is leased by {}",
                    path, existing.owner
                )));
            }
        }
        let expires_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            + ttl_secs;
        let lease = orexis_core::Lease {
            path: path.to_string(),
            owner: owner.to_string(),
            expires_at,
        };
        leases.insert(path.to_string(), lease.clone());
        Ok(lease)
    }

    async fn steal(&self, path: &str, owner: &str) -> PortResult<Option<orexis_core::Lease>> {
        let mut leases = self.leases.lock().expect("leases");
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        match leases.get(path) {
            Some(existing) if existing.expires_at > now => Ok(None),
            _ => {
                let lease = orexis_core::Lease {
                    path: path.to_string(),
                    owner: owner.to_string(),
                    expires_at: now + 60,
                };
                leases.insert(path.to_string(), lease.clone());
                Ok(Some(lease))
            }
        }
    }

    async fn lease_of(&self, path: &str) -> PortResult<Option<orexis_core::Lease>> {
        Ok(self.leases.lock().expect("leases").get(path).cloned())
    }
}
