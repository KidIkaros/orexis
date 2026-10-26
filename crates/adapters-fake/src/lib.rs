//! Deterministic test adapters for the vertical slice. No I/O.

use std::sync::Mutex;
use std::time::Duration;

use async_trait::async_trait;
use orexis_core::{
    Action, EventRef, Message, Observation, ProviderResponse, SessionEvent, StoredEvent, ToolSpec,
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
    ) -> PortResult<ProviderResponse> {
        tokio::time::sleep(self.delay).await;
        Ok(ProviderResponse {
            actions: vec![Action::Done {
                summary: "slow".to_string(),
            }],
            raw_text: "slow".to_string(),
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
    ) -> PortResult<ProviderResponse> {
        let actions = self.script.get(step).cloned().unwrap_or_else(|| {
            vec![Action::Done {
                summary: "script exhausted".to_string(),
            }]
        });
        Ok(ProviderResponse {
            actions,
            raw_text: String::new(),
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
