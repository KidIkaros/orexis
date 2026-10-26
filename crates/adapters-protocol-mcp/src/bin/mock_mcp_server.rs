//! A scripted MCP server for conformance fixtures — the mock_lsp pattern.
//!
//! Scenarios (argv, combinable):
//! - `notify-first` — emits a notification before answering tools/list.
//! - `server-request` — sends a sampling/createMessage request before
//!   answering tools/call, and waits for the refusal.
//! - `stderr` — writes ~200KB of noise to stderr, ending with a marker.

use std::process::ExitCode;

use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

async fn reply(writer: &mut tokio::io::Stdout, id: &Value, result: Value) {
    let line = json!({"jsonrpc": "2.0", "id": id, "result": result}).to_string() + "\n";
    writer.write_all(line.as_bytes()).await.expect("reply");
}

async fn notify(writer: &mut tokio::io::Stdout, method: &str) {
    let line = json!({"jsonrpc": "2.0", "method": method}).to_string() + "\n";
    writer.write_all(line.as_bytes()).await.expect("notify");
    writer.flush().await.expect("flush");
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let scenarios: Vec<String> = std::env::args().skip(1).collect();
    let notify_first = scenarios.iter().any(|scenario| scenario == "notify-first");
    let server_request = scenarios
        .iter()
        .any(|scenario| scenario == "server-request");
    if scenarios.iter().any(|scenario| scenario == "stderr") {
        for _ in 0..2000 {
            eprintln!("{}", "x".repeat(100));
        }
        eprintln!("STDERR-TAIL-MARKER");
    }

    let mut reader = BufReader::new(tokio::io::stdin());
    let mut writer = tokio::io::stdout();
    let mut line = String::new();
    let mut deferred_call: Option<Value> = None;

    loop {
        line.clear();
        if reader.read_line(&mut line).await.expect("read") == 0 {
            return ExitCode::SUCCESS;
        }
        let Ok(message) = serde_json::from_str::<Value>(line.trim()) else {
            continue;
        };
        let method = message.get("method").and_then(Value::as_str).unwrap_or("");
        let id = message.get("id").cloned();
        match (method, id) {
            ("initialize", Some(id)) => {
                reply(
                    &mut writer,
                    &id,
                    json!({
                        "protocolVersion": "2025-06-18",
                        "capabilities": {"tools": {}},
                        "serverInfo": {"name": "mock", "version": "0.0.0"}
                    }),
                )
                .await;
            }
            ("notifications/initialized", _) => {}
            ("tools/list", Some(id)) => {
                if notify_first {
                    notify(&mut writer, "notifications/message").await;
                }
                reply(
                    &mut writer,
                    &id,
                    json!({"tools": [
                        {"name": "mock_tool", "description": "a mock tool",
                         "inputSchema": {"type": "object"}}
                    ]}),
                )
                .await;
            }
            ("tools/call", Some(id)) => {
                if server_request {
                    let request = json!({
                        "jsonrpc": "2.0", "id": 100,
                        "method": "sampling/createMessage", "params": {}
                    });
                    writer
                        .write_all((request.to_string() + "\n").as_bytes())
                        .await
                        .expect("server request");
                    writer.flush().await.expect("flush");
                    deferred_call = Some(id);
                } else {
                    reply(
                        &mut writer,
                        &id,
                        json!({"content": [{"type": "text", "text": "mock says hi"}]}),
                    )
                    .await;
                }
            }
            ("ping", Some(id)) => {
                reply(&mut writer, &id, json!({})).await;
            }
            _ => {
                if let Some(deferred) = deferred_call.take() {
                    // The refusal for our server request arrived; answer the
                    // deferred call now.
                    reply(
                        &mut writer,
                        &deferred,
                        json!({"content": [{"type": "text", "text": "mock says hi"}]}),
                    )
                    .await;
                }
            }
        }
        writer.flush().await.expect("flush");
    }
}
