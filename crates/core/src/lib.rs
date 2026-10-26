//! Pure domain types for the vertical slice. No I/O, no ports, no runtime.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    User,
    Assistant,
    Tool,
    System,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub text: String,
    #[serde(default)]
    pub tool_call_id: Option<String>,
    #[serde(default)]
    pub tool_calls: Vec<ToolCall>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Action {
    Say {
        text: String,
    },
    CallTool {
        call_id: String,
        name: String,
        args: String,
    },
    Done {
        summary: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolCall {
    pub call_id: String,
    pub name: String,
    pub args: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Observation {
    pub name: String,
    pub output: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderResponse {
    pub actions: Vec<Action>,
    pub raw_text: String,
}

/// A tool the model may call. `schema` is a JSON Schema object.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub schema: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodecId {
    Native,
    XmlManual,
    Json,
    Lenient,
}

impl CodecId {
    pub fn name(self) -> &'static str {
        match self {
            Self::Native => "native",
            Self::XmlManual => "xml-manual",
            Self::Json => "json",
            Self::Lenient => "lenient",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextFile {
    pub path: String,
    pub content: String,
}

pub const FORMAT_VERSION: u32 = 1;

/// A durable event reference returned by an append.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventRef {
    pub seq: u64,
    pub event_id: String,
}

/// An event plus its stored metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredEvent {
    pub seq: u64,
    pub event_id: String,
    pub generation_id: u64,
    pub event: SessionEvent,
}

/// The durable session log (ADR-0001), reduced to the slice's event kinds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SessionEvent {
    SessionStart {
        session_id: String,
        format_version: u32,
    },
    UserMessage {
        text: String,
    },
    ModelRequest {
        profile_id: String,
        request_hash: String,
    },
    ModelResponse {
        text: String,
        actions: Vec<Action>,
    },
    ToolCall {
        call_id: String,
        name: String,
        args: String,
    },
    Observation {
        call_id: String,
        name: String,
        output: String,
    },
    Done {
        summary: String,
    },
    Error {
        message: String,
    },
    Reprompt {
        note: String,
    },
    Compaction {
        range_start_seq: u64,
        range_end_seq: u64,
        summary: String,
    },
}

impl SessionEvent {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::SessionStart { .. } => "session_start",
            Self::UserMessage { .. } => "user_message",
            Self::ModelRequest { .. } => "model_request",
            Self::ModelResponse { .. } => "model_response",
            Self::ToolCall { .. } => "tool_call",
            Self::Observation { .. } => "observation",
            Self::Done { .. } => "done",
            Self::Error { .. } => "error",
            Self::Reprompt { .. } => "reprompt",
            Self::Compaction { .. } => "compaction",
        }
    }
}

/// Pure projection: the model-visible messages derived from the log.
pub fn derive_messages(events: &[SessionEvent]) -> Vec<Message> {
    let mut out = Vec::new();
    for event in events {
        match event {
            SessionEvent::UserMessage { text } => out.push(Message {
                role: Role::User,
                text: text.clone(),
                tool_call_id: None,
                tool_calls: Vec::new(),
            }),
            SessionEvent::ModelResponse { text, actions } => {
                let tool_calls: Vec<ToolCall> = actions
                    .iter()
                    .filter_map(|action| match action {
                        Action::CallTool {
                            call_id,
                            name,
                            args,
                        } => Some(ToolCall {
                            call_id: call_id.clone(),
                            name: name.clone(),
                            args: args.clone(),
                        }),
                        _ => None,
                    })
                    .collect();
                if !text.is_empty() || !tool_calls.is_empty() {
                    out.push(Message {
                        role: Role::Assistant,
                        text: text.clone(),
                        tool_call_id: None,
                        tool_calls,
                    });
                }
            }
            SessionEvent::Observation {
                call_id,
                name,
                output,
            } => out.push(Message {
                role: Role::Tool,
                text: format!("{name}: {output}"),
                tool_call_id: Some(call_id.clone()),
                tool_calls: Vec::new(),
            }),
            SessionEvent::Reprompt { note } => out.push(Message {
                role: Role::User,
                text: format!("[reprompt] {note}"),
                tool_call_id: None,
                tool_calls: Vec::new(),
            }),
            SessionEvent::Compaction { summary, .. } => {
                out.clear();
                out.push(Message {
                    role: Role::System,
                    text: format!("[summary] {summary}"),
                    tool_call_id: None,
                    tool_calls: Vec::new(),
                });
            }
            _ => {}
        }
    }
    out
}

/// A stable textual rendering, used to compare a live request with its replay.
pub fn render_messages(messages: &[Message]) -> String {
    messages
        .iter()
        .map(|m| format!("{:?}: {}", m.role, m.text))
        .collect::<Vec<_>>()
        .join("\n")
}

/// A content digest over the model-visible messages.
pub fn hash_messages(messages: &[Message]) -> String {
    let mut hasher = blake3::Hasher::new();
    for message in messages {
        hasher.update(format!("{:?}:{}\n", message.role, message.text).as_bytes());
    }
    hasher.finalize().to_hex().to_string()
}

/// Redact credential-shaped tokens at write time (ADR-0001).
pub fn redact_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for token in text.split_inclusive(char::is_whitespace) {
        let trimmed = token.trim_end();
        if looks_secret(trimmed) {
            let digest = blake3::hash(trimmed.as_bytes()).to_hex().to_string();
            out.push_str(&format!("[REDACTED:{}]", &digest[..8]));
            out.push_str(&token[trimmed.len()..]);
        } else {
            out.push_str(token);
        }
    }
    out
}

fn looks_secret(token: &str) -> bool {
    let candidate = token.trim_matches(|c: char| !c.is_alphanumeric() && c != '-' && c != '_');
    (candidate.starts_with("sk-") && candidate.len() >= 20)
        || (candidate.starts_with("ghp_") && candidate.len() >= 20)
        || (candidate.starts_with("xoxb-") && candidate.len() >= 20)
        || (candidate.starts_with("AKIA") && candidate.len() == 20)
}

pub fn redact_event(event: &SessionEvent) -> SessionEvent {
    match event {
        SessionEvent::SessionStart {
            session_id,
            format_version,
        } => SessionEvent::SessionStart {
            session_id: session_id.clone(),
            format_version: *format_version,
        },
        SessionEvent::UserMessage { text } => SessionEvent::UserMessage {
            text: redact_text(text),
        },
        SessionEvent::ModelRequest {
            profile_id,
            request_hash,
        } => SessionEvent::ModelRequest {
            profile_id: profile_id.clone(),
            request_hash: request_hash.clone(),
        },
        SessionEvent::ModelResponse { text, actions } => SessionEvent::ModelResponse {
            text: redact_text(text),
            actions: actions.iter().map(redact_action).collect(),
        },
        SessionEvent::ToolCall {
            call_id,
            name,
            args,
        } => SessionEvent::ToolCall {
            call_id: call_id.clone(),
            name: name.clone(),
            args: redact_text(args),
        },
        SessionEvent::Observation {
            call_id,
            name,
            output,
        } => SessionEvent::Observation {
            call_id: call_id.clone(),
            name: name.clone(),
            output: redact_text(output),
        },
        SessionEvent::Done { summary } => SessionEvent::Done {
            summary: redact_text(summary),
        },
        SessionEvent::Error { message } => SessionEvent::Error {
            message: redact_text(message),
        },
        SessionEvent::Reprompt { note } => SessionEvent::Reprompt {
            note: redact_text(note),
        },
        SessionEvent::Compaction {
            range_start_seq,
            range_end_seq,
            summary,
        } => SessionEvent::Compaction {
            range_start_seq: *range_start_seq,
            range_end_seq: *range_end_seq,
            summary: redact_text(summary),
        },
    }
}

fn redact_action(action: &Action) -> Action {
    match action {
        Action::Say { text } => Action::Say {
            text: redact_text(text),
        },
        Action::CallTool {
            call_id,
            name,
            args,
        } => Action::CallTool {
            call_id: call_id.clone(),
            name: name.clone(),
            args: redact_text(args),
        },
        Action::Done { summary } => Action::Done {
            summary: redact_text(summary),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derive_skips_control_events() {
        let events = vec![
            SessionEvent::SessionStart {
                session_id: "s".into(),
                format_version: FORMAT_VERSION,
            },
            SessionEvent::UserMessage { text: "hi".into() },
            SessionEvent::ModelResponse {
                text: "hello".into(),
                actions: vec![Action::Say {
                    text: "hello".into(),
                }],
            },
            SessionEvent::ToolCall {
                call_id: "c1".into(),
                name: "read".into(),
                args: "x".into(),
            },
            SessionEvent::Observation {
                call_id: "c1".into(),
                name: "read".into(),
                output: "ok".into(),
            },
        ];
        let messages = derive_messages(&events);
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0].role, Role::User);
        assert_eq!(messages[1].role, Role::Assistant);
        assert_eq!(messages[2].role, Role::Tool);
    }

    #[test]
    fn redacts_credential_shaped_tokens() {
        let event = SessionEvent::UserMessage {
            text: "use sk-abcdefghijklmnopqrstuvwx now".into(),
        };
        let redacted = redact_event(&event);
        let SessionEvent::UserMessage { text } = redacted else {
            panic!("wrong variant");
        };
        assert!(text.contains("[REDACTED:"));
        assert!(!text.contains("sk-abcdefghijklmnopqrstuvwx"));
    }

    #[test]
    fn error_events_are_control_events() {
        let event = SessionEvent::Error {
            message: "boom".to_string(),
        };
        assert_eq!(event.kind(), "error");
        assert!(derive_messages(std::slice::from_ref(&event)).is_empty());
    }

    #[test]
    fn derive_pairs_tool_calls_and_results() {
        let events = vec![
            SessionEvent::ModelResponse {
                text: "reading".into(),
                actions: vec![Action::CallTool {
                    call_id: "c1".into(),
                    name: "read".into(),
                    args: "{}".into(),
                }],
            },
            SessionEvent::Observation {
                call_id: "c1".into(),
                name: "read".into(),
                output: "ok".into(),
            },
        ];
        let messages = derive_messages(&events);
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].tool_calls[0].call_id, "c1");
        assert_eq!(messages[1].tool_call_id.as_deref(), Some("c1"));
    }

    #[test]
    fn compaction_resets_history() {
        let events = vec![
            SessionEvent::UserMessage { text: "old".into() },
            SessionEvent::Compaction {
                range_start_seq: 1,
                range_end_seq: 1,
                summary: "earlier".into(),
            },
            SessionEvent::UserMessage { text: "new".into() },
        ];
        let messages = derive_messages(&events);
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, Role::System);
        assert_eq!(messages[1].text, "new");
    }

    #[test]
    fn hash_is_stable() {
        let messages = vec![Message {
            role: Role::User,
            text: "hi".into(),
            tool_call_id: None,
            tool_calls: Vec::new(),
        }];
        assert_eq!(hash_messages(&messages), hash_messages(&messages));
    }
}
