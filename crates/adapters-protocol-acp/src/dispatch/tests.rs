use super::*;
use orexis_adapters_jsonrpc::{Message, RequestId};

#[test]
fn initialize_parses_and_responds_with_capabilities() {
    let incoming = parse_incoming(
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":1,"clientCapabilities":{}}}"#,
    )
    .expect("parse");
    assert_eq!(
        incoming,
        AcpIncoming::Initialize {
            id: RequestId::Number(1),
        }
    );
    let response = initialize_response(RequestId::Number(1));
    let value = match &response {
        Message::Response(response) => response.to_value(),
        _ => panic!("not a response"),
    };
    assert_eq!(
        value,
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "protocolVersion": 1,
                "agentCapabilities": {"loadSession": false},
            },
        })
    );
}

#[test]
fn session_new_parses_and_responds_with_the_session_id() {
    let incoming = parse_incoming(
        r#"{"jsonrpc":"2.0","id":2,"method":"session/new","params":{"cwd":"/tmp","mcpServers":[]}}"#,
    )
    .expect("parse");
    assert_eq!(
        incoming,
        AcpIncoming::SessionNew {
            id: RequestId::Number(2),
        }
    );
    let response = session_new_response(RequestId::Number(2), "s-1");
    let value = match &response {
        Message::Response(response) => response.to_value(),
        _ => panic!("not a response"),
    };
    assert_eq!(
        value,
        serde_json::json!({"jsonrpc": "2.0", "id": 2, "result": {"sessionId": "s-1"}})
    );
}

#[test]
fn session_prompt_extracts_the_text_blocks() {
    let incoming = parse_incoming(
        r#"{"jsonrpc":"2.0","id":3,"method":"session/prompt","params":{"sessionId":"s-1","prompt":[{"type":"text","text":"fix"},{"type":"text","text":"the build"}]}}"#,
    )
    .expect("parse");
    assert_eq!(
        incoming,
        AcpIncoming::SessionPrompt {
            id: RequestId::Number(3),
            session_id: "s-1".into(),
            text: "fix\nthe build".into(),
        }
    );
    let response = prompt_response(RequestId::Number(3), "end_turn");
    let value = match &response {
        Message::Response(response) => response.to_value(),
        _ => panic!("not a response"),
    };
    assert_eq!(
        value,
        serde_json::json!({"jsonrpc": "2.0", "id": 3, "result": {"stopReason": "end_turn"}})
    );
}

#[test]
fn session_cancel_is_a_notification() {
    let incoming = parse_incoming(
        r#"{"jsonrpc":"2.0","method":"session/cancel","params":{"sessionId":"s-1"}}"#,
    )
    .expect("parse");
    assert_eq!(
        incoming,
        AcpIncoming::SessionCancel {
            session_id: "s-1".into(),
        }
    );
}

#[test]
fn unknown_methods_are_unhandled_and_answered() {
    let incoming =
        parse_incoming(r#"{"jsonrpc":"2.0","id":9,"method":"session/load","params":{}}"#)
            .expect("parse");
    assert_eq!(
        incoming,
        AcpIncoming::Unhandled {
            id: Some(RequestId::Number(9)),
            method: "session/load".into(),
        }
    );
    let response = super::method_not_found(Some(RequestId::Number(9)), "session/load");
    let value = match &response {
        Message::Response(response) => response.to_value(),
        _ => panic!("not a response"),
    };
    assert_eq!(value["error"]["code"], -32601);
}

#[test]
fn a_response_from_the_client_is_a_client_response() {
    let incoming = parse_incoming(
        r#"{"jsonrpc":"2.0","id":7,"result":{"outcome":{"outcome":"selected","optionId":"allow_once"}}}"#,
    )
    .expect("parse");
    assert_eq!(
        incoming,
        AcpIncoming::ClientResponse {
            id: Some(RequestId::Number(7)),
            result: Some(serde_json::json!({
                "outcome": {"outcome": "selected", "optionId": "allow_once"}
            })),
            error: None,
        }
    );
}

#[test]
fn outcomes_map_to_yes_always() {
    let selected =
        serde_json::json!({"outcome": {"outcome": "selected", "optionId": "allow_once"}});
    assert_eq!(outcome_from(Some(&selected), None), (true, false));
    let always =
        serde_json::json!({"outcome": {"outcome": "selected", "optionId": "allow_always"}});
    assert_eq!(outcome_from(Some(&always), None), (true, true));
    let rejected =
        serde_json::json!({"outcome": {"outcome": "selected", "optionId": "reject_once"}});
    assert_eq!(outcome_from(Some(&rejected), None), (false, false));
    let cancelled = serde_json::json!({"outcome": {"outcome": "cancelled"}});
    assert_eq!(outcome_from(Some(&cancelled), None), (false, false));
    assert_eq!(outcome_from(None, None), (false, false));
}

#[test]
fn permission_requests_carry_the_tool_and_fixed_option_ids() {
    let message = super::permission_request(RequestId::Number(4), "s-1", 12, "write_file");
    let value = match &message {
        Message::Request(request) => request.to_value(),
        _ => panic!("not a request"),
    };
    assert_eq!(value["method"], "session/request_permission");
    assert_eq!(value["params"]["toolCall"]["id"], "12");
    assert_eq!(value["params"]["toolCall"]["title"], "write_file");
    let options = value["params"]["options"].as_array().expect("options");
    assert_eq!(options.len(), 3);
    assert_eq!(options[0]["id"], "allow_once");
    assert_eq!(options[2]["id"], "reject_once");
}

#[test]
fn update_notifications_carry_the_session_and_update() {
    let message = super::update_notification(
        "s-1",
        serde_json::json!({"agentMessageChunk": {"content": {"type": "text", "text": "hi"}}}),
    );
    let value = match &message {
        Message::Notification(notification) => notification.to_value(),
        _ => panic!("not a notification"),
    };
    assert_eq!(value["method"], "session/update");
    assert_eq!(value["params"]["sessionId"], "s-1");
    assert_eq!(
        value["params"]["update"]["agentMessageChunk"]["content"]["text"],
        "hi"
    );
}
