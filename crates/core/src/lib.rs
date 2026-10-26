//! Pure domain types for the vertical slice. No I/O, no ports, no runtime.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    pub role: Role,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Say(String),
    CallTool { name: String, args: String },
    Done { summary: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Observation {
    pub name: String,
    pub output: String,
}

pub const FORMAT_VERSION: u32 = 1;

/// The durable session log (ADR-0001), reduced to the slice's event kinds.
#[derive(Debug, Clone, PartialEq, Eq)]
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
        request_hash: u64,
    },
    ModelResponse {
        text: String,
        actions: Vec<Action>,
    },
    ToolCall {
        name: String,
        args: String,
    },
    Observation {
        name: String,
        output: String,
    },
    Done {
        summary: String,
    },
}

/// Pure projection: the model-visible messages derived from the log.
pub fn derive_messages(events: &[SessionEvent]) -> Vec<Message> {
    let mut out = Vec::new();
    for event in events {
        match event {
            SessionEvent::UserMessage { text } => out.push(Message {
                role: Role::User,
                text: text.clone(),
            }),
            SessionEvent::ModelResponse { text, .. } if !text.is_empty() => out.push(Message {
                role: Role::Assistant,
                text: text.clone(),
            }),
            SessionEvent::Observation { name, output } => out.push(Message {
                role: Role::Tool,
                text: format!("{name}: {output}"),
            }),
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
                actions: vec![Action::Say("hello".into())],
            },
            SessionEvent::ToolCall {
                name: "read".into(),
                args: "x".into(),
            },
            SessionEvent::Observation {
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
}
