//! Conformance fixtures: the stdio shell against the scripted mock server.
//! No live third-party peer; the real-peer check is manual (M7-03).

use std::time::Duration;

use orexis_adapters_protocol_mcp::{McpServerConfig, McpStdioClient};

const TIMEOUT: Duration = Duration::from_secs(5);

fn mock(args: &[&str]) -> McpServerConfig {
    McpServerConfig {
        name: "mock".into(),
        command: env!("CARGO_BIN_EXE_mock_mcp_server").into(),
        args: args.iter().map(|arg| arg.to_string()).collect(),
        env: Default::default(),
    }
}

#[tokio::test]
async fn handshake_list_and_call_over_a_real_process() {
    let mut client = McpStdioClient::spawn(&mock(&[]), TIMEOUT)
        .await
        .expect("spawn");
    let (tools, next_cursor) = client.list_tools(None).await.expect("list");
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "mock_tool");
    assert_eq!(next_cursor, None);

    let call = client
        .call_tool("mock_tool", Some(serde_json::json!({"path": "a.rs"})))
        .await
        .expect("call");
    assert!(call.success);
    assert_eq!(call.content[0].as_text(), Some("mock says hi"));

    client.ping().await.expect("ping");
}

#[tokio::test]
async fn spawn_failure_is_an_error_not_a_hang() {
    let config = McpServerConfig {
        name: "missing".into(),
        command: "/nonexistent/mcp-server-xyz".into(),
        args: Vec::new(),
        env: Default::default(),
    };
    let error = McpStdioClient::spawn(&config, TIMEOUT)
        .await
        .expect_err("spawn failure");
    assert!(error.to_string().contains("transport"), "error: {error}");
}

#[tokio::test]
async fn a_silent_server_times_out() {
    let config = McpServerConfig {
        name: "silent".into(),
        command: "sh".into(),
        args: vec!["-c".into(), "sleep 10".into()],
        env: Default::default(),
    };
    let error = McpStdioClient::spawn(&config, Duration::from_millis(300))
        .await
        .expect_err("timeout");
    assert_eq!(error, orexis_adapters_protocol_mcp::McpError::Timeout);
}

#[tokio::test]
async fn stderr_is_captured_and_capped() {
    let mut client = McpStdioClient::spawn(&mock(&["stderr"]), TIMEOUT)
        .await
        .expect("spawn");
    // Drain a round trip so the capture task has time to run.
    let _ = client.list_tools(None).await.expect("list");
    let stderr = client.stderr_text();
    assert!(!stderr.is_empty());
    assert!(stderr.len() <= 64 * 1024 + 256, "len {}", stderr.len());
    assert!(stderr.contains("STDERR-TAIL-MARKER"), "tail: {stderr}");
}

#[tokio::test]
async fn an_interleaved_notification_does_not_break_the_round_trip() {
    let mut client = McpStdioClient::spawn(&mock(&["notify-first"]), TIMEOUT)
        .await
        .expect("spawn");
    let (tools, _) = client.list_tools(None).await.expect("list");
    assert_eq!(tools[0].name, "mock_tool");
}

#[tokio::test]
async fn a_server_request_is_refused_without_deadlock() {
    let mut client = McpStdioClient::spawn(&mock(&["server-request"]), TIMEOUT)
        .await
        .expect("spawn");
    let call = client
        .call_tool("mock_tool", None)
        .await
        .expect("call completes after refusing sampling");
    assert_eq!(call.content[0].as_text(), Some("mock says hi"));
}
