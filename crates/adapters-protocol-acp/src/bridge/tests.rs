use super::*;
use orexis_protocol::SessionEvent;

#[test]
fn a_delta_becomes_an_agent_message_chunk() {
    let update = update_from(
        &ServerMsg::Delta {
            text: "hello".into(),
        },
        "s-1",
    )
    .expect("update");
    let value = match &update {
        Message::Notification(notification) => notification.to_value(),
        _ => panic!("not a notification"),
    };
    assert_eq!(value["params"]["sessionId"], "s-1");
    assert_eq!(
        value["params"]["update"]["agentMessageChunk"]["content"],
        serde_json::json!({"type": "text", "text": "hello"})
    );
}

#[test]
fn a_tool_call_becomes_a_tool_call_update_in_progress() {
    let update = update_from(
        &ServerMsg::Event {
            seq: 1,
            replay: false,
            event: SessionEvent::ToolCall {
                call_id: "c1".into(),
                name: "read_file".into(),
                args: "{}".into(),
            },
        },
        "s-1",
    )
    .expect("update");
    let value = match &update {
        Message::Notification(notification) => notification.to_value(),
        _ => panic!("not a notification"),
    };
    let tool_call = &value["params"]["update"]["toolCall"];
    assert_eq!(tool_call["id"], "c1");
    assert_eq!(tool_call["title"], "read_file");
    assert_eq!(tool_call["kind"], "read");
    assert_eq!(tool_call["status"], "in_progress");
}

#[test]
fn an_observation_completes_the_tool_call() {
    let update = update_from(
        &ServerMsg::Event {
            seq: 2,
            replay: false,
            event: SessionEvent::Observation {
                call_id: "c1".into(),
                name: "read_file".into(),
                output: "the file body".into(),
            },
        },
        "s-1",
    )
    .expect("update");
    let value = match &update {
        Message::Notification(notification) => notification.to_value(),
        _ => panic!("not a notification"),
    };
    let tool_call = &value["params"]["update"]["toolCallUpdate"];
    assert_eq!(tool_call["id"], "c1");
    assert_eq!(tool_call["status"], "completed");
    assert_eq!(tool_call["rawOutput"], "the file body");
}

#[test]
fn shell_and_edit_kinds_map() {
    let kind = |name: &str| {
        let update = update_from(
            &ServerMsg::Event {
                seq: 1,
                replay: false,
                event: SessionEvent::ToolCall {
                    call_id: "c".into(),
                    name: name.into(),
                    args: "{}".into(),
                },
            },
            "s",
        )
        .expect("update");
        match &update {
            Message::Notification(notification) => notification.to_value()["params"]["update"]
                ["toolCall"]["kind"]
                .as_str()
                .expect("kind")
                .to_string(),
            _ => panic!("not a notification"),
        }
    };
    assert_eq!(kind("shell"), "execute");
    assert_eq!(kind("edit_file"), "edit");
    assert_eq!(kind("mcp/fs/read_file"), "other");
}

#[test]
fn done_and_interrupted_carry_stop_reasons() {
    assert_eq!(
        stop_reason_from(&ServerMsg::Event {
            seq: 1,
            replay: false,
            event: SessionEvent::Done {
                summary: "done".into(),
            },
        }),
        Some("end_turn")
    );
    assert_eq!(
        stop_reason_from(&ServerMsg::Event {
            seq: 2,
            replay: false,
            event: SessionEvent::Interrupted {
                reason: "client interrupt".into(),
            },
        }),
        Some("cancelled")
    );
}

#[test]
fn an_unpaired_error_becomes_a_visible_chunk() {
    let update = update_from(
        &ServerMsg::Event {
            seq: 3,
            replay: false,
            event: SessionEvent::Error {
                message: "missing 'path'".into(),
            },
        },
        "s-1",
    )
    .expect("update");
    let value = match &update {
        Message::Notification(notification) => notification.to_value(),
        _ => panic!("not a notification"),
    };
    assert_eq!(
        value["params"]["update"]["agentMessageChunk"]["content"]["text"],
        "[error] missing 'path'"
    );
}

#[test]
fn non_update_messages_produce_nothing() {
    assert_eq!(
        update_from(
            &ServerMsg::Audit {
                entries: Vec::new()
            },
            "s-1"
        ),
        None
    );
    assert_eq!(
        stop_reason_from(&ServerMsg::Event {
            seq: 1,
            replay: false,
            event: SessionEvent::UserMessage { text: "hi".into() },
        }),
        None
    );
}
