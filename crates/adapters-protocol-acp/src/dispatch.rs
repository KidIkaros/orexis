//! ACP method dispatch: parse an incoming line, build outgoing responses.

use super::PROTOCOL_VERSION;
use orexis_adapters_jsonrpc::{
    ErrorObject, Message, Notification, ParseError, Request, RequestId, Response,
};
use serde_json::{json, Value};

pub const METHOD_NOT_FOUND: i64 = -32601;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AcpError {
    #[error("malformed ACP wire: {0}")]
    Parse(#[from] ParseError),
    #[error("{0}")]
    Shape(String),
}

/// A client request or notification, dispatched by method.
#[derive(Debug, Clone, PartialEq)]
pub enum AcpIncoming {
    Initialize {
        id: RequestId,
    },
    SessionNew {
        id: RequestId,
    },
    SessionPrompt {
        id: RequestId,
        session_id: String,
        text: String,
    },
    SessionCancel {
        session_id: String,
    },
    /// The client's answer to an agent-side request (permissions, M7-07).
    ClientResponse {
        id: Option<RequestId>,
        result: Option<Value>,
        error: Option<ErrorObject>,
    },
    Unhandled {
        id: Option<RequestId>,
        method: String,
    },
}

/// `prompt` is a list of content blocks; the text blocks are the prompt.
fn prompt_text(params: &Value) -> String {
    params
        .get("prompt")
        .and_then(Value::as_array)
        .map(|blocks| {
            blocks
                .iter()
                .filter_map(|block| block.get("text").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default()
}

pub fn parse_incoming(line: &str) -> Result<AcpIncoming, AcpError> {
    let message = orexis_adapters_jsonrpc::parse_line(line)?;
    match message {
        Message::Request(request) => Ok(match request.method.as_str() {
            "initialize" => AcpIncoming::Initialize { id: request.id },
            "session/new" => AcpIncoming::SessionNew { id: request.id },
            "session/prompt" => {
                let params = request.params.unwrap_or(Value::Null);
                AcpIncoming::SessionPrompt {
                    id: request.id,
                    session_id: params
                        .get("sessionId")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    text: prompt_text(&params),
                }
            }
            _ => AcpIncoming::Unhandled {
                id: Some(request.id),
                method: request.method,
            },
        }),
        Message::Notification(notification) => Ok(match notification.method.as_str() {
            "session/cancel" => AcpIncoming::SessionCancel {
                session_id: notification
                    .params
                    .unwrap_or(Value::Null)
                    .get("sessionId")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
            },
            _ => AcpIncoming::Unhandled {
                id: None,
                method: notification.method,
            },
        }),
        Message::Response(Response { id, result, error }) => {
            Ok(AcpIncoming::ClientResponse { id, result, error })
        }
    }
}

pub fn initialize_response(id: RequestId) -> Message {
    Message::Response(Response::result(
        id,
        json!({
            "protocolVersion": PROTOCOL_VERSION,
            "agentCapabilities": {
                "loadSession": false,
            },
        }),
    ))
}

pub fn session_new_response(id: RequestId, session_id: &str) -> Message {
    Message::Response(Response::result(id, json!({ "sessionId": session_id })))
}

pub fn prompt_response(id: RequestId, stop_reason: &str) -> Message {
    Message::Response(Response::result(id, json!({ "stopReason": stop_reason })))
}

pub fn method_not_found(id: Option<RequestId>, method: &str) -> Message {
    Message::Response(Response::error(
        id,
        ErrorObject {
            code: METHOD_NOT_FOUND,
            message: format!("method not supported: {method}"),
            data: None,
        },
    ))
}

/// A `session/update` notification carrying one update object.
pub fn update_notification(session_id: &str, update: Value) -> Message {
    Message::Notification(Notification {
        method: "session/update".to_string(),
        params: Some(json!({ "sessionId": session_id, "update": update })),
    })
}

/// An agent-side request to the client (used by M7-07 for permissions).
pub fn client_request(id: RequestId, method: &str, params: Value) -> Message {
    Message::Request(Request {
        id,
        method: method.to_string(),
        params: Some(params),
    })
}

/// The `session/request_permission` request for a pending tool approval.
/// Option ids are fixed so the client's answer maps back deterministically.
pub fn permission_request(
    id: RequestId,
    session_id: &str,
    approval_id: u64,
    tool: &str,
) -> Message {
    client_request(
        id,
        "session/request_permission",
        json!({
            "sessionId": session_id,
            "toolCall": {
                "id": approval_id.to_string(),
                "title": tool,
                "kind": "other",
                "status": "pending",
            },
            "options": [
                { "id": "allow_once", "kind": "allow_once", "name": "Allow once" },
                { "id": "allow_always", "kind": "allow_always", "name": "Allow always" },
                { "id": "reject_once", "kind": "reject_once", "name": "Reject" },
            ],
        }),
    )
}

/// The client's permission answer as `(yes, always)`; a cancelled or
/// errored outcome denies.
pub fn outcome_from(result: Option<&Value>, error: Option<&ErrorObject>) -> (bool, bool) {
    if error.is_some() {
        return (false, false);
    }
    let Some(result) = result else {
        return (false, false);
    };
    let outcome = result.get("outcome").cloned().unwrap_or(Value::Null);
    if outcome.get("outcome").and_then(Value::as_str) != Some("selected") {
        return (false, false);
    }
    match outcome.get("optionId").and_then(Value::as_str) {
        Some("allow_once") => (true, false),
        Some("allow_always") => (true, true),
        _ => (false, false),
    }
}

#[cfg(test)]
mod tests;
