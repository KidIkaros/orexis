//! A2A client core — sans-io (ADR-0017 §1, §6).
//!
//! The A2A wire is JSON-RPC 2.0 with A2A's own methods and objects: an
//! agent card describes a remote agent, `message/send` hands it work, and
//! `tasks/get` reads the task back. This crate keeps the shape of that
//! protocol in plain data: parsing a card, building requests, and reading
//! responses. The HTTP shell owns the socket.
//!
//! Two method namings exist in the wild. A2A v1.0 uses PascalCase
//! (`SendMessage`, `GetTask`); v0.3 and earlier used the slash form
//! (`message/send`, `tasks/get`). The card says which protocol the peer
//! speaks, so the core picks the naming from the card rather than
//! guessing.

use orexis_adapters_jsonrpc::{ErrorObject, Message, Request, RequestId, Response};
use serde_json::{json, Value};

/// The A2A method naming a peer speaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MethodNaming {
    /// A2A v1.0: `SendMessage`, `GetTask`.
    PascalCase,
    /// A2A v0.3 and earlier: `message/send`, `tasks/get`.
    Slash,
}

impl MethodNaming {
    pub fn send_message(&self) -> &'static str {
        match self {
            Self::PascalCase => "SendMessage",
            Self::Slash => "message/send",
        }
    }

    pub fn get_task(&self) -> &'static str {
        match self {
            Self::PascalCase => "GetTask",
            Self::Slash => "tasks/get",
        }
    }

    /// The streaming variant of sending a message. The spec gives it its
    /// own method rather than a content-type switch.
    pub fn send_streaming_message(&self) -> &'static str {
        match self {
            Self::PascalCase => "SendStreamingMessage",
            Self::Slash => "message/stream",
        }
    }

    /// The naming a card implies. A card that names no protocol version
    /// predates v1.0, so the slash form is the safe default.
    pub fn from_protocol_version(version: &str) -> Self {
        if version.starts_with("1.") {
            Self::PascalCase
        } else {
            Self::Slash
        }
    }
}

/// A remote agent's card: who it is and what it can do.
#[derive(Debug, Clone, PartialEq)]
pub struct AgentCard {
    pub name: String,
    pub description: String,
    /// The endpoint the agent serves JSON-RPC on.
    pub url: String,
    pub version: String,
    /// The A2A protocol version the peer speaks.
    pub protocol_version: String,
    /// True when the peer can stream task updates.
    pub streaming: bool,
    /// True when the peer accepts push notification configuration.
    pub push_notifications: bool,
    /// The skills the agent advertises, as `(id, name, description)`.
    pub skills: Vec<AgentSkill>,
}

/// One advertised capability.
#[derive(Debug, Clone, PartialEq)]
pub struct AgentSkill {
    pub id: String,
    pub name: String,
    pub description: String,
}

impl AgentCard {
    /// The method naming this peer speaks.
    pub fn naming(&self) -> MethodNaming {
        MethodNaming::from_protocol_version(&self.protocol_version)
    }
}

/// Where a task stands. Terminal states end the interaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Submitted,
    Working,
    InputRequired,
    AuthRequired,
    Completed,
    Failed,
    Canceled,
    Rejected,
    /// A state this client does not know; the raw value is kept.
    Unknown,
}

impl TaskState {
    /// Parse a `TASK_STATE_*` value.
    pub fn parse(value: &str) -> Self {
        match value {
            "TASK_STATE_SUBMITTED" => Self::Submitted,
            "TASK_STATE_WORKING" => Self::Working,
            "TASK_STATE_INPUT_REQUIRED" => Self::InputRequired,
            "TASK_STATE_AUTH_REQUIRED" => Self::AuthRequired,
            "TASK_STATE_COMPLETED" => Self::Completed,
            "TASK_STATE_FAILED" => Self::Failed,
            "TASK_STATE_CANCELED" => Self::Canceled,
            "TASK_STATE_REJECTED" => Self::Rejected,
            _ => Self::Unknown,
        }
    }

    /// True when the task will not change again.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Failed | Self::Canceled | Self::Rejected
        )
    }

    /// The raw `TASK_STATE_*` value.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Submitted => "TASK_STATE_SUBMITTED",
            Self::Working => "TASK_STATE_WORKING",
            Self::InputRequired => "TASK_STATE_INPUT_REQUIRED",
            Self::AuthRequired => "TASK_STATE_AUTH_REQUIRED",
            Self::Completed => "TASK_STATE_COMPLETED",
            Self::Failed => "TASK_STATE_FAILED",
            Self::Canceled => "TASK_STATE_CANCELED",
            Self::Rejected => "TASK_STATE_REJECTED",
            Self::Unknown => "TASK_STATE_UNKNOWN",
        }
    }
}

/// A task the remote agent is tracking.
#[derive(Debug, Clone, PartialEq)]
pub struct Task {
    pub id: String,
    pub context_id: Option<String>,
    pub state: TaskState,
    /// The raw `state` value, for an unknown state.
    pub raw_state: String,
    /// The text the agent produced, if any.
    pub text: String,
}

/// The outcome of a `message/send` or `tasks/get`.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    /// The agent answered directly with a message, no task involved.
    Message { text: String },
    /// The agent created or continued a task.
    Task(Task),
}

/// The A2A client core: build requests, read responses.
pub struct A2aCore {
    naming: MethodNaming,
    next_id: i64,
    next_message: u64,
    endpoint: String,
}

impl A2aCore {
    pub fn new(naming: MethodNaming, endpoint: impl Into<String>) -> Self {
        Self {
            naming,
            next_id: 1,
            next_message: 1,
            endpoint: endpoint.into(),
        }
    }

    /// The endpoint this client talks to.
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    pub fn naming(&self) -> MethodNaming {
        self.naming
    }

    /// A `message/send` request carrying one text part. The messageId is
    /// generated here: peers may refuse a repeated messageId as a replay,
    /// so it must never be derived from the text alone.
    pub fn send_message(&mut self, text: &str) -> (RequestId, Message) {
        let message_id = self.next_message_id();
        let id = RequestId::Number(self.next_id);
        self.next_id += 1;
        let request = Request {
            id: id.clone(),
            method: self.naming.send_message().to_string(),
            params: Some(json!({
                "message": {
                    "role": "ROLE_USER",
                    "parts": [{"text": text}],
                    "messageId": message_id,
                }
            })),
        };
        (id, Message::Request(request))
    }

    /// A `tasks/get` request for one task id.
    pub fn get_task(&mut self, task_id: &str) -> (RequestId, Message) {
        let id = RequestId::Number(self.next_id);
        self.next_id += 1;
        let request = Request {
            id: id.clone(),
            method: self.naming.get_task().to_string(),
            params: Some(json!({ "id": task_id })),
        };
        (id, Message::Request(request))
    }

    /// A streaming `message/stream` request. The peer answers with an SSE
    /// stream whose frames carry the task and its updates.
    pub fn stream_message(&mut self, text: &str) -> (RequestId, Message) {
        let message_id = self.next_message_id();
        let id = RequestId::Number(self.next_id);
        self.next_id += 1;
        let request = Request {
            id: id.clone(),
            method: self.naming.send_streaming_message().to_string(),
            params: Some(json!({
                "message": {
                    "role": "ROLE_USER",
                    "parts": [{"text": text}],
                    "messageId": message_id,
                }
            })),
        };
        (id, Message::Request(request))
    }

    /// A fresh messageId; the counter keeps same-length messages distinct.
    fn next_message_id(&mut self) -> String {
        let id = format!("msg-{}", self.next_message);
        self.next_message += 1;
        id
    }

    /// Read a `message/send` or `tasks/get` result.
    pub fn read_result(&self, result: &Value) -> Result<Outcome, A2aError> {
        if let Some(task) = result.get("task") {
            return Ok(Outcome::Task(read_task(task)));
        }
        if let Some(message) = result.get("message") {
            return Ok(Outcome::Message {
                text: read_text(message),
            });
        }
        // Some peers answer with the task object inline rather than
        // wrapped; recognize it by its own fields.
        if result.get("id").is_some() && result.get("status").is_some() {
            return Ok(Outcome::Task(read_task(result)));
        }
        Err(A2aError::Shape(
            "the result carries neither a task nor a message".into(),
        ))
    }
}

/// A remote agent's answer to one request.
#[derive(Debug, Clone, PartialEq)]
pub enum A2aEvent {
    /// The request succeeded; the result is in `result`.
    Result { id: RequestId, result: Value },
    /// The request failed.
    Failed { id: RequestId, error: ErrorObject },
    /// The peer sent something else (a notification, or a response to a
    /// request we did not make).
    Other(Message),
}

impl A2aEvent {
    /// The id of the request this event answers, when it answers one.
    pub fn request_id(&self) -> Option<&RequestId> {
        match self {
            Self::Result { id, .. } | Self::Failed { id, .. } => Some(id),
            Self::Other(_) => None,
        }
    }
}

/// Read one JSON-RPC message as an event.
pub fn read_message(message: Message) -> A2aEvent {
    match message {
        Message::Response(Response {
            id: Some(id),
            result: Some(result),
            error: None,
        }) => A2aEvent::Result { id, result },
        Message::Response(Response {
            id: Some(id),
            result: None,
            error: Some(error),
        }) => A2aEvent::Failed { id, error },
        Message::Response(Response {
            id: None,
            result: None,
            error: Some(error),
        }) => A2aEvent::Failed {
            id: RequestId::Number(0),
            error,
        },
        other => A2aEvent::Other(other),
    }
}

/// The task object of a result.
fn read_task(value: &Value) -> Task {
    let status = value.get("status");
    let raw_state = status
        .and_then(|status| status.get("state"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let text = status
        .and_then(|status| status.get("message"))
        .map(read_text)
        .unwrap_or_default();
    Task {
        id: value
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        context_id: value
            .get("contextId")
            .and_then(Value::as_str)
            .map(str::to_string),
        state: TaskState::parse(&raw_state),
        raw_state,
        text,
    }
}

/// The text of a message object: its text parts, joined.
fn read_text(value: &Value) -> String {
    let parts = value.get("parts").and_then(Value::as_array);
    match parts {
        Some(parts) => parts
            .iter()
            .filter_map(|part| part.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join("\n"),
        None => value
            .get("text")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
    }
}

/// Parse an agent card.
pub fn parse_card(value: &Value) -> Result<AgentCard, A2aError> {
    let name = value
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    if name.is_empty() {
        return Err(A2aError::Shape("the agent card has no name".into()));
    }
    let url = card_url(value);
    if url.is_empty() {
        return Err(A2aError::Shape(
            "the agent card names no JSON-RPC endpoint".into(),
        ));
    }
    let capabilities = value.get("capabilities");
    let protocol_version = value
        .get("protocolVersion")
        .and_then(Value::as_str)
        .or_else(|| {
            value
                .get("supportedInterfaces")
                .and_then(Value::as_array)
                .and_then(|interfaces| interfaces.first())
                .and_then(|interface| interface.get("protocolVersion"))
                .and_then(Value::as_str)
        })
        .unwrap_or_default()
        .to_string();
    let skills = value
        .get("skills")
        .and_then(Value::as_array)
        .map(|skills| {
            skills
                .iter()
                .map(|skill| AgentSkill {
                    id: skill
                        .get("id")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    name: skill
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    description: skill
                        .get("description")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(AgentCard {
        name,
        description: value
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        url,
        version: value
            .get("version")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        protocol_version,
        streaming: capabilities
            .and_then(|caps| caps.get("streaming"))
            .and_then(Value::as_bool)
            .unwrap_or(false),
        push_notifications: capabilities
            .and_then(|caps| caps.get("pushNotifications"))
            .and_then(Value::as_bool)
            .unwrap_or(false),
        skills,
    })
}

/// The endpoint a card advertises. v1.0 cards list interfaces; older
/// cards carry a top-level `url`.
fn card_url(value: &Value) -> String {
    if let Some(url) = value.get("url").and_then(Value::as_str) {
        if !url.is_empty() {
            return url.to_string();
        }
    }
    value
        .get("supportedInterfaces")
        .and_then(Value::as_array)
        .and_then(|interfaces| {
            interfaces
                .iter()
                .find(|interface| {
                    interface
                        .get("protocolBinding")
                        .and_then(Value::as_str)
                        .map(|binding| binding.eq_ignore_ascii_case("JSONRPC"))
                        .unwrap_or(false)
                })
                .or_else(|| interfaces.first())
        })
        .and_then(|interface| interface.get("url"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

/// A2A client errors.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum A2aError {
    #[error("malformed A2A wire: {0}")]
    Parse(#[from] orexis_adapters_jsonrpc::ParseError),
    #[error("{0}")]
    Shape(String),
    #[error("transport: {0}")]
    Transport(String),
    #[error("the peer answered HTTP {0}")]
    Status(u16),
    #[error("the peer has no agent card")]
    NoCard,
    #[error("the peer does not support {0}")]
    Unsupported(String),
}
