//! Bridge: daemon `ServerMsg` → ACP `session/update` notifications and
//! stop reasons. Pure; the ACP bin feeds it the broadcast stream.

use orexis_adapters_jsonrpc::Message;
use orexis_protocol::ServerMsg;
use serde_json::json;

/// The ACP tool-call kind for a built-in tool name.
fn kind_of(name: &str) -> &'static str {
    match name {
        "read_file" | "list_dir" | "search" | "git_status" | "git_diff" => "read",
        "write_file" | "edit_file" => "edit",
        "shell" | "verify" => "execute",
        _ => "other",
    }
}

/// The `session/update` notification for a stream message, if any.
pub fn update_from(message: &ServerMsg, session_id: &str) -> Option<Message> {
    let update = match message {
        ServerMsg::Delta { text } => json!({
            "agentMessageChunk": { "content": { "type": "text", "text": text } },
        }),
        ServerMsg::Event { event, .. } => match event {
            orexis_protocol::SessionEvent::ToolCall {
                call_id,
                name,
                args,
            } => json!({
                "toolCall": {
                    "id": call_id,
                    "title": name,
                    "kind": kind_of(name),
                    "status": "in_progress",
                    "rawInput": args,
                },
            }),
            orexis_protocol::SessionEvent::Observation {
                call_id, output, ..
            } => json!({
                "toolCallUpdate": {
                    "id": call_id,
                    "status": "completed",
                    "rawOutput": output,
                },
            }),
            // An error that is not paired with a tool call is still
            // model-visible history; the ACP client sees it as a chunk.
            orexis_protocol::SessionEvent::Error { message } => json!({
                "agentMessageChunk": {
                    "content": { "type": "text", "text": format!("[error] {message}") },
                },
            }),
            _ => return None,
        },
        _ => return None,
    };
    Some(super::dispatch::update_notification(session_id, update))
}

/// The ACP stop reason carried by a terminal stream message.
pub fn stop_reason_from(message: &ServerMsg) -> Option<&'static str> {
    match message {
        ServerMsg::Event { event, .. } => match event {
            orexis_protocol::SessionEvent::Done { .. } => Some("end_turn"),
            orexis_protocol::SessionEvent::Interrupted { .. } => Some("cancelled"),
            _ => None,
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests;
