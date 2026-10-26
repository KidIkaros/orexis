use super::*;

#[test]
fn request_round_trips_number_and_string_ids() {
    for id in [RequestId::Number(1), RequestId::String("a".into())] {
        let request = Request {
            id: id.clone(),
            method: "initialize".into(),
            params: Some(serde_json::json!({"protocolVersion": "2025-06-18"})),
        };
        let parsed = parse_line(&request.to_value().to_string()).expect("parse");
        assert_eq!(parsed, Message::Request(request));
    }
}

#[test]
fn request_without_params_omits_the_member() {
    let request = Request {
        id: RequestId::Number(2),
        method: "ping".into(),
        params: None,
    };
    let value = request.to_value();
    assert!(value.get("params").is_none());
    let parsed = parse_line(&value.to_string()).expect("parse");
    assert_eq!(parsed, Message::Request(request));
}

#[test]
fn notification_parses_without_an_id() {
    let parsed =
        parse_line(r#"{"jsonrpc":"2.0","method":"notify","params":{"x":1}}"#).expect("parse");
    assert_eq!(
        parsed,
        Message::Notification(Notification {
            method: "notify".into(),
            params: Some(serde_json::json!({"x": 1})),
        })
    );
}

#[test]
fn null_id_is_a_notification() {
    let parsed = parse_line(r#"{"jsonrpc":"2.0","id":null,"method":"notify"}"#).expect("parse");
    assert!(matches!(parsed, Message::Notification(_)));
}

#[test]
fn params_null_is_absent() {
    let parsed =
        parse_line(r#"{"jsonrpc":"2.0","id":1,"method":"ping","params":null}"#).expect("parse");
    assert_eq!(
        parsed,
        Message::Request(Request {
            id: RequestId::Number(1),
            method: "ping".into(),
            params: None,
        })
    );
}

#[test]
fn result_response_round_trips() {
    let response = Response::result(RequestId::Number(7), serde_json::json!({"ok": true}));
    let parsed = parse_line(&response.to_value().to_string()).expect("parse");
    assert_eq!(parsed, Message::Response(response));
}

#[test]
fn null_result_stays_present() {
    let parsed = parse_line(r#"{"jsonrpc":"2.0","id":1,"result":null}"#).expect("parse");
    assert_eq!(
        parsed,
        Message::Response(Response {
            id: Some(RequestId::Number(1)),
            result: Some(Value::Null),
            error: None,
        })
    );
}

#[test]
fn error_response_round_trips_with_data() {
    let response = Response::error(
        Some(RequestId::String("a".into())),
        ErrorObject {
            code: -32601,
            message: "method not found".into(),
            data: Some(serde_json::json!({"method": "x"})),
        },
    );
    let parsed = parse_line(&response.to_value().to_string()).expect("parse");
    assert_eq!(parsed, Message::Response(response));
}

#[test]
fn error_response_may_carry_a_null_id() {
    let parsed = parse_line(
        r#"{"jsonrpc":"2.0","id":null,"error":{"code":-32700,"message":"parse error"}}"#,
    )
    .expect("parse");
    assert_eq!(
        parsed,
        Message::Response(Response {
            id: None,
            result: None,
            error: Some(ErrorObject {
                code: -32700,
                message: "parse error".into(),
                data: None,
            }),
        })
    );
}

#[test]
fn malformed_lines_error_without_panicking() {
    let cases = [
        (r#"[]"#, ParseError::NotAnObject("[]".into())),
        (r#"{}"#, ParseError::Version),
        (r#"{"jsonrpc":"1.0","method":"x"}"#, ParseError::Version),
        (r#"{"jsonrpc":"2.0"}"#, ParseError::Shapeless),
        (
            r#"{"jsonrpc":"2.0","method":"x","result":1}"#,
            ParseError::Mixed,
        ),
        (
            r#"{"jsonrpc":"2.0","id":1,"result":1,"error":{"code":1,"message":"m"}}"#,
            ParseError::BothResultAndError,
        ),
        (
            r#"{"jsonrpc":"2.0","id":1.5,"method":"x"}"#,
            ParseError::Id("1.5".into()),
        ),
        (
            r#"{"jsonrpc":"2.0","id":1,"error":{"message":"no code"}}"#,
            ParseError::ErrorObject("missing code".into()),
        ),
    ];
    for (line, expected) in cases {
        let parsed = parse_line(line);
        assert_eq!(parsed.err().map(|error| error.to_string()), {
            let expected = expected.to_string();
            Some(expected)
        });
    }
}

#[test]
fn invalid_json_is_the_json_variant() {
    assert!(matches!(parse_line("not json"), Err(ParseError::Json(_))));
}

#[test]
fn error_object_needs_a_message() {
    let parsed = parse_line(r#"{"jsonrpc":"2.0","id":1,"error":{"code":1}}"#);
    assert!(parsed.is_err());
}
