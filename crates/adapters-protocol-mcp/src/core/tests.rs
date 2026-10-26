use super::*;
use crate::types::ClientInfo;

fn core() -> McpCore {
    McpCore::new(ClientInfo {
        name: "orexis".into(),
        version: "0.0.0".into(),
    })
}

/// Drive the recorded handshake: initialize → response → initialized.
fn handshake() -> McpCore {
    let mut core = core();
    core.initialize().expect("initialize");
    let events = core
        .receive(
            r#"{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2025-06-18","capabilities":{"tools":{}},"serverInfo":{"name":"fs","version":"1.0"}}}"#,
        )
        .expect("initialize response");
    assert_eq!(
        events,
        vec![Event::Initialized(Initialization {
            protocol_version: "2025-06-18".into(),
            server_info: ServerInfo {
                name: "fs".into(),
                version: "1.0".into(),
            },
            capabilities: ServerCapabilities {
                tools: Some(serde_json::json!({})),
                resources: None,
                prompts: None,
            },
        })]
    );
    while core.pop_outgoing().is_some() {}
    core
}

fn outgoing_line(core: &mut McpCore) -> String {
    let message = core.pop_outgoing().expect("outgoing");
    orexis_adapters_jsonrpc::encode_line(&message)
}

#[test]
fn handshake_sends_initialize_then_initialized() {
    let mut core = core();
    core.initialize().expect("initialize");
    let line = outgoing_line(&mut core);
    let sent: serde_json::Value = serde_json::from_str(line.trim()).expect("json");
    assert_eq!(
        sent,
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {"name": "orexis", "version": "0.0.0"},
            },
        })
    );
    let _ = core.receive(
        r#"{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2025-06-18","capabilities":{},"serverInfo":{"name":"fs","version":"1.0"}}}"#,
    );
    let line = outgoing_line(&mut core);
    let sent: serde_json::Value = serde_json::from_str(line.trim()).expect("json");
    assert_eq!(
        sent,
        serde_json::json!({"jsonrpc": "2.0", "method": "notifications/initialized"})
    );
    assert!(core.pop_outgoing().is_none());
}

#[test]
fn requests_before_initialize_are_rejected() {
    let mut core = core();
    assert_eq!(core.list_tools(None), Err(McpError::NotReady));
    assert_eq!(core.call_tool("read_file", None), Err(McpError::NotReady));
    assert_eq!(core.ping(), Err(McpError::NotReady));
}

#[test]
fn initialize_twice_is_rejected() {
    let mut core = handshake();
    assert_eq!(core.initialize(), Err(McpError::AlreadyInitializing));
}

#[test]
fn tools_list_parses_and_paginates() {
    let mut core = handshake();
    core.list_tools(None).expect("list");
    assert!(outgoing_line(&mut core).contains(r#""method":"tools/list""#));
    let events = core
        .receive(
            r#"{"jsonrpc":"2.0","id":2,"result":{"tools":[{"name":"read_file","description":"Read a file","inputSchema":{"type":"object"}}],"nextCursor":"page2"}}"#,
        )
        .expect("response");
    assert_eq!(
        events,
        vec![Event::ToolsListed {
            tools: vec![orexis_core::ToolSpec {
                name: "read_file".into(),
                description: "Read a file".into(),
                schema: serde_json::json!({"type": "object"}),
            }],
            next_cursor: Some("page2".into()),
        }]
    );
    core.list_tools(Some("page2".into())).expect("list page 2");
    let line = outgoing_line(&mut core);
    assert!(line.contains(r#""params":{"cursor":"page2"}"#));
}

#[test]
fn tool_call_parses_content_and_is_error() {
    let mut core = handshake();
    core.call_tool("read_file", Some(serde_json::json!({"path": "a.rs"})))
        .expect("call");
    let line = outgoing_line(&mut core);
    assert!(line.contains(r#""method":"tools/call""#));
    assert!(line.contains(r#""arguments":{"path":"a.rs"}"#));

    let events = core
        .receive(
            r#"{"jsonrpc":"2.0","id":2,"result":{"content":[{"type":"text","text":"hello"}],"isError":false}}"#,
        )
        .expect("response");
    assert_eq!(
        events,
        vec![Event::ToolCalled(ToolCallResult {
            success: true,
            content: vec![crate::types::ToolContent::Text {
                text: "hello".into(),
            }],
            is_error: false,
        })]
    );

    core.call_tool("read_file", None).expect("call");
    let _ = outgoing_line(&mut core);
    let events = core
        .receive(
            r#"{"jsonrpc":"2.0","id":3,"result":{"content":[{"type":"text","text":"missing"}],"isError":true}}"#,
        )
        .expect("response");
    assert_eq!(
        events,
        vec![Event::ToolCalled(ToolCallResult {
            success: false,
            content: vec![crate::types::ToolContent::Text {
                text: "missing".into(),
            }],
            is_error: true,
        })]
    );
}

#[test]
fn a_json_rpc_error_response_becomes_failed_with_the_method() {
    let mut core = handshake();
    core.list_tools(None).expect("list");
    let _ = outgoing_line(&mut core);
    let events = core
        .receive(r#"{"jsonrpc":"2.0","id":2,"error":{"code":-32601,"message":"method not found"}}"#)
        .expect("response");
    assert_eq!(
        events,
        vec![Event::Failed {
            method: "tools/list".into(),
            error: ErrorObject {
                code: -32601,
                message: "method not found".into(),
                data: None,
            },
        }]
    );
}

#[test]
fn resources_list_and_read_parse() {
    let mut core = handshake();
    core.list_resources(None).expect("list");
    let _ = outgoing_line(&mut core);
    let events = core
        .receive(
            r#"{"jsonrpc":"2.0","id":2,"result":{"resources":[{"uri":"file:///a.txt","name":"a","mimeType":"text/plain","description":"d"}]}}"#,
        )
        .expect("response");
    assert_eq!(
        events,
        vec![Event::ResourcesListed {
            resources: vec![crate::types::McpResource {
                uri: "file:///a.txt".into(),
                name: "a".into(),
                mime_type: "text/plain".into(),
                description: "d".into(),
            }],
            next_cursor: None,
        }]
    );

    core.read_resource("file:///a.txt").expect("read");
    let line = outgoing_line(&mut core);
    assert!(line.contains(r#""method":"resources/read""#));
    assert!(line.contains(r#""params":{"uri":"file:///a.txt"}"#));
    let events = core
        .receive(
            r#"{"jsonrpc":"2.0","id":3,"result":{"contents":[{"uri":"file:///a.txt","mimeType":"text/plain","text":"body"}]}}"#,
        )
        .expect("response");
    assert_eq!(
        events,
        vec![Event::ResourceRead {
            contents: vec![crate::types::ResourceContent {
                uri: "file:///a.txt".into(),
                mime_type: "text/plain".into(),
                text: Some("body".into()),
                blob: None,
            }],
        }]
    );
}

#[test]
fn prompts_list_and_get_parse() {
    let mut core = handshake();
    core.list_prompts(None).expect("list");
    let _ = outgoing_line(&mut core);
    let events = core
        .receive(
            r#"{"jsonrpc":"2.0","id":2,"result":{"prompts":[{"name":"summarize","description":"d","arguments":[{"name":"text","description":"t","required":true}]}]}}"#,
        )
        .expect("response");
    assert_eq!(
        events,
        vec![Event::PromptsListed {
            prompts: vec![crate::types::McpPrompt {
                name: "summarize".into(),
                description: "d".into(),
                arguments: vec![crate::types::McpPromptArgument {
                    name: "text".into(),
                    description: "t".into(),
                    required: true,
                }],
            }],
            next_cursor: None,
        }]
    );

    core.get_prompt("summarize", Some(serde_json::json!({"text": "hi"})))
        .expect("get");
    let line = outgoing_line(&mut core);
    assert!(line.contains(r#""method":"prompts/get""#));
    let events = core
        .receive(
            r#"{"jsonrpc":"2.0","id":3,"result":{"messages":[{"role":"user","content":{"type":"text","text":"summarize: hi"}}]}}"#,
        )
        .expect("response");
    assert_eq!(
        events,
        vec![Event::PromptGot(serde_json::json!({
            "messages": [
                {"role": "user", "content": {"type": "text", "text": "summarize: hi"}}
            ]
        }))]
    );
}

#[test]
fn ping_pongs() {
    let mut core = handshake();
    core.ping().expect("ping");
    assert!(outgoing_line(&mut core).contains(r#""method":"ping""#));
    let events = core
        .receive(r#"{"jsonrpc":"2.0","id":2,"result":{}}"#)
        .expect("response");
    assert_eq!(events, vec![Event::Pong]);
}

#[test]
fn a_server_request_is_emitted_and_can_be_refused() {
    let mut core = handshake();
    let events = core
        .receive(r#"{"jsonrpc":"2.0","id":9,"method":"sampling/createMessage","params":{}}"#)
        .expect("server request");
    assert_eq!(
        events,
        vec![Event::ServerRequest {
            id: RequestId::Number(9),
            method: "sampling/createMessage".into(),
            params: Some(serde_json::json!({})),
        }]
    );
    core.refuse(RequestId::Number(9));
    let line = outgoing_line(&mut core);
    assert!(line.contains(r#""id":9"#));
    assert!(line.contains(r#""code":-32601"#));
}

#[test]
fn a_server_request_can_be_answered() {
    let mut core = handshake();
    let _ = core.receive(r#"{"jsonrpc":"2.0","id":9,"method":"roots/list"}"#);
    core.respond(RequestId::Number(9), serde_json::json!({"roots": []}));
    let line = outgoing_line(&mut core);
    assert!(line.contains(r#""id":9"#));
    assert!(line.contains(r#""result":{"roots":[]}"#));
}

#[test]
fn interleaved_traffic_correlates_by_id() {
    let mut core = handshake();
    core.list_tools(None).expect("list");
    core.call_tool("read_file", None).expect("call");

    let _ = outgoing_line(&mut core);
    let _ = outgoing_line(&mut core);

    // The server interleaves: a notification, then the responses out of
    // order. Kindred's blocking read broke here; correlation must not.
    let events = core
        .receive(r#"{"jsonrpc":"2.0","method":"notifications/message","params":{"level":"info"}}"#)
        .expect("notification");
    assert_eq!(
        events,
        vec![Event::ServerNotification {
            method: "notifications/message".into(),
            params: Some(serde_json::json!({"level": "info"})),
        }]
    );
    let events = core
        .receive(r#"{"jsonrpc":"2.0","id":3,"result":{"content":[],"isError":false}}"#)
        .expect("tool response");
    assert!(matches!(events.as_slice(), [Event::ToolCalled(_)]));
    let events = core
        .receive(
            r#"{"jsonrpc":"2.0","id":2,"result":{"tools":[{"name":"read_file","description":"","inputSchema":{"type":"object"}}]}}"#,
        )
        .expect("tools response");
    assert!(matches!(events.as_slice(), [Event::ToolsListed { .. }]));
}

#[test]
fn a_stray_response_is_an_event_not_an_error() {
    let mut core = handshake();
    let events = core
        .receive(r#"{"jsonrpc":"2.0","id":99,"result":{}}"#)
        .expect("stray");
    assert!(matches!(events.as_slice(), [Event::Stray(_)]));
}

#[test]
fn list_changed_notifications_become_events() {
    let mut core = handshake();
    for (method, expected) in [
        ("notifications/tools/list_changed", Event::ToolsChanged),
        (
            "notifications/resources/list_changed",
            Event::ResourcesChanged,
        ),
        ("notifications/prompts/list_changed", Event::PromptsChanged),
    ] {
        let events = core
            .receive(&format!(r#"{{"jsonrpc":"2.0","method":"{method}"}}"#))
            .expect("notification");
        assert_eq!(events, vec![expected]);
    }
}

#[test]
fn a_malformed_line_is_an_error() {
    let mut core = handshake();
    assert!(matches!(core.receive("garbage"), Err(McpError::Parse(_))));
}

#[test]
fn ids_increase_monotonically() {
    let mut core = handshake();
    core.list_tools(None).expect("list");
    let first = outgoing_line(&mut core);
    core.call_tool("read_file", None).expect("call");
    let second = outgoing_line(&mut core);
    let first_id: i64 = serde_json::from_str::<serde_json::Value>(&first).expect("json")["id"]
        .as_i64()
        .expect("id");
    let second_id: i64 = serde_json::from_str::<serde_json::Value>(&second).expect("json")["id"]
        .as_i64()
        .expect("id");
    assert_eq!(first_id, 2);
    assert_eq!(second_id, 3);
}
