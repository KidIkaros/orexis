//! Conformance fixtures: the streamable HTTP shell against a recorded
//! HTTP transcript. No live peer; the real-peer check is manual (M7-05).
//!
//! The transcript is explicit about the wire: every POST the client makes
//! is matched with the response the server sends, in order. The handshake
//! is two POSTs — the `initialize` request and the `notifications/
//! initialized` notification the client owes the server — so each scripted
//! sequence carries the 202 ack for the notification.

use std::sync::Arc;
use std::time::Duration;

use orexis_adapters_http::{MockResponse, MockTransport};
use orexis_adapters_protocol_mcp::{McpError, McpHttpClient, McpHttpConfig};

const TIMEOUT: Duration = Duration::from_secs(5);

const INIT_RESULT: &str = r#"{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2025-06-18","capabilities":{"tools":{}},"serverInfo":{"name":"fs","version":"1.0"}}}"#;

fn config() -> McpHttpConfig {
    McpHttpConfig::new("fs", "https://mcp.example.com/mcp")
}

/// The server's answer to the initialization request, carrying the
/// session it assigned.
fn init_result() -> MockResponse {
    MockResponse::json(INIT_RESULT).with_header("mcp-session-id", "sess-1")
}

/// The server's answer to the initialized notification: 202, no body.
fn accepted() -> MockResponse {
    MockResponse::json("").with_status(202)
}

fn client(responses: Vec<MockResponse>) -> (McpHttpClient, Arc<MockTransport>) {
    let transport = Arc::new(MockTransport::new(Vec::new()).with_responses(responses));
    let client = McpHttpClient::new(config(), transport.clone(), TIMEOUT);
    (client, transport)
}

/// The bodies the client posted, in order.
fn posted(transport: &MockTransport) -> Vec<String> {
    transport
        .calls
        .lock()
        .expect("calls")
        .iter()
        .map(|(_, _, body)| body.clone())
        .collect()
}

#[tokio::test]
async fn the_handshake_records_the_session_and_the_protocol_version() {
    let (mut client, transport) = client(vec![init_result(), accepted()]);
    let initialization = client.connect().await.expect("connect");
    assert_eq!(initialization.protocol_version, "2025-06-18");
    assert_eq!(initialization.server_info.name, "fs");
    assert_eq!(client.session_id(), Some("sess-1"));
    assert_eq!(client.protocol_version(), Some("2025-06-18"));

    let bodies = posted(&transport);
    assert_eq!(
        bodies.len(),
        2,
        "initialize then the initialized notification"
    );
    assert!(
        bodies[0].contains("\"method\":\"initialize\""),
        "{}",
        bodies[0]
    );
    assert!(
        bodies[1].contains("notifications/initialized"),
        "{}",
        bodies[1]
    );
}

#[tokio::test]
async fn later_requests_carry_the_session_and_the_protocol_version() {
    let (mut client, transport) = client(vec![
        init_result(),
        accepted(),
        MockResponse::json(
            r#"{"jsonrpc":"2.0","id":2,"result":{"tools":[{"name":"read_file","description":"read","inputSchema":{"type":"object"}}]}}"#,
        ),
    ]);
    client.connect().await.expect("connect");
    let (tools, next) = client.list_tools(None).await.expect("list");
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "read_file");
    assert_eq!(next, None);

    // The headers of each call, in order: the initialize request carries
    // no session yet, the later ones must carry both.
    let history = transport.call_headers.lock().expect("call_headers").clone();
    assert_eq!(history.len(), 3, "initialize, the ack, then tools/list");
    assert!(
        !history[0].iter().any(|(name, _)| name == "mcp-session-id"),
        "the initialize request has no session yet: {:?}",
        history[0]
    );
    for (index, headers) in history.iter().enumerate().skip(1) {
        assert!(
            headers
                .iter()
                .any(|(name, value)| name == "mcp-session-id" && value == "sess-1"),
            "call {index} must carry the session: {headers:?}"
        );
        assert!(
            headers
                .iter()
                .any(|(name, value)| name == "mcp-protocol-version" && value == "2025-06-18"),
            "call {index} must carry the protocol version: {headers:?}"
        );
    }
}

#[tokio::test]
async fn an_event_stream_response_parses_through_the_same_core() {
    let (mut client, _transport) = client(vec![
        MockResponse::sse(format!("event: message\ndata: {INIT_RESULT}\n\n")),
        accepted(),
        MockResponse::sse(
            "event: message\ndata: {\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{\"tools\":[{\"name\":\"read_file\",\"inputSchema\":{\"type\":\"object\"}}]}}\n\n"
                .to_string(),
        ),
    ]);
    client.connect().await.expect("connect over SSE");
    let (tools, _) = client.list_tools(None).await.expect("list over SSE");
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "read_file");
}

#[tokio::test]
async fn a_server_request_inside_the_stream_is_answered() {
    // The server interleaves a request before its response; the shell
    // refuses it once, and the refusal is a third POST.
    let (mut client, transport) = client(vec![
        init_result(),
        accepted(),
        MockResponse::sse(
            concat!(
                "event: message\ndata: {\"jsonrpc\":\"2.0\",\"id\":100,\"method\":\"sampling/createMessage\",\"params\":{}}\n\n",
                "event: message\ndata: {\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{\"tools\":[]}}\n\n"
            )
            .to_string(),
        ),
        accepted(),
    ]);
    client.connect().await.expect("connect");
    let (tools, _) = client.list_tools(None).await.expect("list");
    assert!(tools.is_empty());

    let bodies = posted(&transport);
    let refusal = bodies
        .iter()
        .find(|body| body.contains("\"method not found\""))
        .expect("the refusal is sent");
    assert!(refusal.contains("\"jsonrpc\":\"2.0\""), "body: {refusal}");
    assert!(
        bodies
            .iter()
            .all(|body| !body.contains("sampling/createMessage")),
        "the refusal must not echo the request: {bodies:?}"
    );
}

#[tokio::test]
async fn an_expired_session_is_an_error_not_a_silent_reinitialize() {
    let (mut client, transport) = client(vec![
        init_result(),
        accepted(),
        MockResponse::json("{\"error\":\"session gone\"}").with_status(404),
    ]);
    client.connect().await.expect("connect");
    let error = client
        .list_tools(None)
        .await
        .expect_err("an expired session is an error");
    assert_eq!(error, McpError::SessionExpired);

    // No re-initialization was attempted: the exchange stopped.
    let bodies = posted(&transport);
    assert_eq!(
        bodies.len(),
        3,
        "initialize, the ack, then the failed request"
    );
    assert_eq!(
        bodies
            .iter()
            .filter(|body| body.contains("\"method\":\"initialize\""))
            .count(),
        1,
        "the client must not silently re-initialize: {bodies:?}"
    );
}

#[tokio::test]
async fn a_server_error_status_is_a_transport_error() {
    let (mut client, _transport) = client(vec![MockResponse::json("boom").with_status(500)]);
    let error = client.connect().await.expect_err("a 500 is an error");
    assert!(matches!(error, McpError::Transport(_)), "error: {error}");
}

#[tokio::test]
async fn a_tool_call_round_trips_over_http() {
    let (mut client, _transport) = client(vec![
        init_result(),
        accepted(),
        MockResponse::json(
            r#"{"jsonrpc":"2.0","id":2,"result":{"content":[{"type":"text","text":"file body"}]}}"#,
        ),
    ]);
    client.connect().await.expect("connect");
    let result = client
        .call_tool("read_file", Some(serde_json::json!({"path": "a.rs"})))
        .await
        .expect("call");
    assert!(result.success);
    assert_eq!(result.content[0].as_text(), Some("file body"));
}
