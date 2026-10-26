//! Deterministic test adapters for the vertical slice. No I/O.

use std::sync::Mutex;

use async_trait::async_trait;
use orexis_core::{Action, Message, Observation, SessionEvent};
use orexis_ports::{EventSink, Provider, SessionAppend, SessionRead, ToolExecutor};

pub struct ScriptedProvider {
    script: Vec<Vec<Action>>,
}

impl ScriptedProvider {
    pub fn new(script: Vec<Vec<Action>>) -> Self {
        Self { script }
    }
}

#[async_trait]
impl Provider for ScriptedProvider {
    async fn complete(&self, _messages: &[Message], step: usize) -> Vec<Action> {
        self.script.get(step).cloned().unwrap_or_else(|| {
            vec![Action::Done {
                summary: "script exhausted".to_string(),
            }]
        })
    }
}

pub struct EchoTools;

#[async_trait]
impl ToolExecutor for EchoTools {
    async fn execute(&self, name: &str, args: &str) -> Observation {
        Observation {
            name: name.to_string(),
            output: format!("ok({args})"),
        }
    }
}

#[derive(Default)]
pub struct InMemorySession {
    events: Mutex<Vec<SessionEvent>>,
}

#[async_trait]
impl SessionAppend for InMemorySession {
    async fn append(&self, event: SessionEvent) {
        self.events.lock().expect("session lock").push(event);
    }
}

#[async_trait]
impl SessionRead for InMemorySession {
    async fn events(&self) -> Vec<SessionEvent> {
        self.events.lock().expect("session lock").clone()
    }
}

#[derive(Default)]
pub struct RecordingSink {
    lines: Mutex<Vec<String>>,
}

impl RecordingSink {
    pub fn lines(&self) -> Vec<String> {
        self.lines.lock().expect("sink lock").clone()
    }
}

impl EventSink for RecordingSink {
    fn emit(&self, event: &SessionEvent) {
        self.lines
            .lock()
            .expect("sink lock")
            .push(format!("{event:?}"));
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
            SessionEvent::ToolCall { name, args } => out.push_str(&format!("  ▸ {name} {args}\n")),
            SessionEvent::Observation { name, output } => {
                out.push_str(&format!("    {name} → {output}\n"))
            }
            SessionEvent::Done { summary } => out.push_str(&format!("✓ {summary}\n")),
            _ => {}
        }
    }
    out
}
