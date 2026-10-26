use super::*;
use crate::message::{ErrorObject, Message, Notification, Request, RequestId, Response};

#[test]
fn encode_line_appends_a_newline() {
    let message = Message::Notification(Notification {
        method: "notify".into(),
        params: None,
    });
    let line = encode_line(&message);
    assert!(line.ends_with('\n'));
    assert_eq!(line.lines().count(), 1);
}

#[test]
fn decode_str_round_trips_an_encoded_transcript() {
    let transcript = vec![
        Message::Request(Request {
            id: RequestId::Number(1),
            method: "initialize".into(),
            params: Some(serde_json::json!({"protocolVersion": "2025-06-18"})),
        }),
        Message::Response(Response::result(
            RequestId::Number(1),
            serde_json::json!({"capabilities": {}}),
        )),
        Message::Notification(Notification {
            method: "notifications/initialized".into(),
            params: None,
        }),
    ];
    let encoded = transcript.iter().map(encode_line).collect::<String>();
    assert_eq!(decode_str(&encoded).expect("decode"), transcript);
}

#[test]
fn decode_str_skips_blank_and_whitespace_lines() {
    let decoded =
        decode_str("\n  \n{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":null}\n\n").expect("decode");
    assert_eq!(
        decoded,
        vec![Message::Response(Response {
            id: Some(RequestId::Number(1)),
            result: Some(serde_json::Value::Null),
            error: None,
        })]
    );
}

#[test]
fn decode_str_reports_the_line_number_of_a_malformed_line() {
    let error =
        decode_str("{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":null}\n{}\n").expect_err("malformed");
    assert_eq!(
        error,
        FramingError::Malformed {
            line: 2,
            source: crate::message::ParseError::Version,
        }
    );
}

#[test]
fn decode_str_reports_the_line_of_invalid_json() {
    let error = decode_str("\n{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":\n").expect_err("malformed");
    assert!(matches!(
        error,
        FramingError::Malformed {
            line: 2,
            source: crate::message::ParseError::Json(_),
        }
    ));
}

#[test]
fn a_recorded_transcript_decodes_in_order() {
    let recorded = concat!(
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18"}}"#,
        "\n",
        r#"{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2025-06-18","capabilities":{"tools":{}}}}"#,
        "\n",
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        "\n",
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#,
        "\n",
        r#"{"jsonrpc":"2.0","id":2,"result":{"tools":[{"name":"read_file","description":"Read a file"}]}}"#,
        "\n",
    );
    let messages = decode_str(recorded).expect("decode");
    assert_eq!(messages.len(), 5);
    assert!(matches!(&messages[0], Message::Request(request) if request.method == "initialize"));
    assert!(matches!(&messages[1], Message::Response(response) if response.error.is_none()));
    assert!(
        matches!(&messages[2], Message::Notification(notification) if notification.method == "notifications/initialized")
    );
    assert!(matches!(&messages[3], Message::Request(request) if request.method == "tools/list"));
    assert!(matches!(&messages[4], Message::Response(response) if response.error.is_none()));
}

#[test]
fn an_error_response_in_a_transcript_carries_the_error_object() {
    let messages = decode_str(
        r#"{"jsonrpc":"2.0","id":3,"error":{"code":-32601,"message":"method not found","data":{"method":"x"}}}"#,
    )
    .expect("decode");
    assert_eq!(
        messages,
        vec![Message::Response(Response {
            id: Some(RequestId::Number(3)),
            result: None,
            error: Some(ErrorObject {
                code: -32601,
                message: "method not found".into(),
                data: Some(serde_json::json!({"method": "x"})),
            }),
        })]
    );
}
