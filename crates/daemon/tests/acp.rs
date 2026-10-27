//! Conformance fixtures: the ACP server against recorded client lines,
//! driven with the fake runtime. No live peer; the real-peer check is
//! manual (M7-11).

use std::path::Path;
use std::sync::Arc;

use orexis_adapters_codec::NativeCodec;
use orexis_adapters_fake::{ScriptedProvider, SlowProvider};
use orexis_adapters_session_sqlite::SqliteSessionFactory;
use orexis_adapters_tools::LocalTools;
use orexis_core::{Action, PermissionPolicy};
use orexis_daemon::{AcpServer, DaemonConfig};

fn fake_server(dir: &Path, script: Vec<Vec<Action>>) -> AcpServer {
    let config = DaemonConfig {
        socket: dir.join("unused.sock"),
        profile_id: "fake".into(),
        model_id: "fake".into(),
        sessions: Arc::new(SqliteSessionFactory::new(dir.join("sessions.db"))),
        entity: Arc::new(orexis_adapters_entity_sqlite::SqliteEntityFactory::new(
            dir.join("entity.db"),
        )),
        identity_path: None,
        evolution: None,
        external: Vec::new(),
        provider: Arc::new(ScriptedProvider::new(script)),
        tools: Arc::new(LocalTools::new(dir).expect("tools")),
        workspace: Arc::new(LocalTools::new(dir).expect("workspace")),
        codec: Arc::new(NativeCodec),
        policy: Arc::new(PermissionPolicy::standard()),
        checkpointer: None,
        verifier: None,
        system_prompt: String::new(),
        context_tools: Vec::new(),
        context_files: Vec::new(),
        context_limit: 32768,
        max_steps: 8,
        generation_slots: 1,
    };
    AcpServer::new(config)
}

async fn drive(server: &AcpServer, lines: &[&str]) -> String {
    let input = lines
        .iter()
        .map(|line| format!("{line}\n"))
        .collect::<String>();
    let mut reader = tokio::io::BufReader::new(std::io::Cursor::new(input));
    let out = server
        .run(&mut reader, std::io::Cursor::new(Vec::new()))
        .await
        .expect("run");
    String::from_utf8(out.into_inner()).expect("utf8")
}

fn messages(output: &str) -> Vec<serde_json::Value> {
    output
        .lines()
        .map(|line| serde_json::from_str(line).expect("json"))
        .collect()
}

fn session_id_of(output: &str) -> String {
    let parsed = messages(output);
    parsed.last().expect("response")["result"]["sessionId"]
        .as_str()
        .expect("session id")
        .to_string()
}

#[tokio::test]
async fn initialize_and_session_new_respond() {
    let dir = tempfile::tempdir().expect("tempdir");
    let server = fake_server(dir.path(), Vec::new());
    let output = drive(
        &server,
        &[
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":1}}"#,
            r#"{"jsonrpc":"2.0","id":2,"method":"session/new","params":{"cwd":"/tmp"}}"#,
        ],
    )
    .await;
    let parsed = messages(&output);
    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed[0]["id"], 1);
    assert_eq!(parsed[0]["result"]["protocolVersion"], 1);
    assert_eq!(parsed[1]["id"], 2);
    assert!(parsed[1]["result"]["sessionId"].is_string());
}

#[tokio::test]
async fn a_prompt_streams_updates_and_ends_with_a_stop_reason() {
    let dir = tempfile::tempdir().expect("tempdir");
    let script = vec![vec![
        Action::Say {
            text: "working on it".to_string(),
        },
        Action::CallTool {
            call_id: "c1".to_string(),
            name: "list_dir".to_string(),
            args: "{}".to_string(),
        },
        Action::Done {
            summary: "done".to_string(),
        },
    ]];
    let server = fake_server(dir.path(), script);
    let output = drive(
        &server,
        &[
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
            r#"{"jsonrpc":"2.0","id":2,"method":"session/new","params":{}}"#,
        ],
    )
    .await;
    let session_id = session_id_of(&output);

    let prompt = format!(
        r#"{{"jsonrpc":"2.0","id":3,"method":"session/prompt","params":{{"sessionId":"{session_id}","prompt":[{{"type":"text","text":"hi"}}]}}}}"#
    );
    let output = drive(&server, &[&prompt]).await;
    let parsed = messages(&output);
    let updates: Vec<&serde_json::Value> = parsed
        .iter()
        .filter(|message| message["method"] == "session/update")
        .collect();
    assert!(updates
        .iter()
        .any(
            |update| update["params"]["update"]["agentMessageChunk"]["content"]["text"]
                == "working on it"
        ));
    assert!(updates
        .iter()
        .any(|update| update["params"]["update"]["toolCall"]["id"] == "c1"));
    assert!(updates
        .iter()
        .any(|update| update["params"]["update"]["toolCallUpdate"]["status"] == "completed"));
    let response = parsed.last().expect("response");
    assert_eq!(response["id"], 3);
    assert_eq!(response["result"]["stopReason"], "end_turn");
}

#[tokio::test]
async fn cancel_stops_the_prompt_with_cancelled() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config = DaemonConfig {
        socket: dir.path().join("unused.sock"),
        profile_id: "fake".into(),
        model_id: "fake".into(),
        sessions: Arc::new(SqliteSessionFactory::new(dir.path().join("sessions.db"))),
        entity: Arc::new(orexis_adapters_entity_sqlite::SqliteEntityFactory::new(
            dir.path().join("entity.db"),
        )),
        identity_path: None,
        evolution: None,
        external: Vec::new(),
        provider: Arc::new(SlowProvider::new(std::time::Duration::from_secs(30))),
        tools: Arc::new(LocalTools::new(dir.path()).expect("tools")),
        workspace: Arc::new(LocalTools::new(dir.path()).expect("workspace")),
        codec: Arc::new(NativeCodec),
        policy: Arc::new(PermissionPolicy::standard()),
        checkpointer: None,
        verifier: None,
        system_prompt: String::new(),
        context_tools: Vec::new(),
        context_files: Vec::new(),
        context_limit: 32768,
        max_steps: 8,
        generation_slots: 1,
    };
    let server = AcpServer::new(config);
    let output = drive(
        &server,
        &[
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
            r#"{"jsonrpc":"2.0","id":2,"method":"session/new","params":{}}"#,
        ],
    )
    .await;
    let session_id = session_id_of(&output);

    // The drive runs as a task, so the read loop consumes the cancel
    // mid-prompt; the prompt must end cancelled, not hang for 30 seconds.
    let prompt = format!(
        r#"{{"jsonrpc":"2.0","id":3,"method":"session/prompt","params":{{"sessionId":"{session_id}","prompt":[{{"type":"text","text":"hi"}}]}}}}"#
    );
    let cancel = format!(
        r#"{{"jsonrpc":"2.0","method":"session/cancel","params":{{"sessionId":"{session_id}"}}}}"#
    );
    let output = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        drive(&server, &[&prompt, &cancel]),
    )
    .await
    .expect("prompt finished promptly");
    let parsed = messages(&output);
    let response = parsed.last().expect("response");
    assert_eq!(response["id"], 3);
    assert_eq!(response["result"]["stopReason"], "cancelled");
}

#[tokio::test]
async fn unknown_methods_get_method_not_found() {
    let dir = tempfile::tempdir().expect("tempdir");
    let server = fake_server(dir.path(), Vec::new());
    let output = drive(
        &server,
        &[r#"{"jsonrpc":"2.0","id":9,"method":"session/load","params":{}}"#],
    )
    .await;
    let parsed = messages(&output);
    let response = parsed.last().expect("response");
    assert_eq!(response["error"]["code"], -32601);
}

#[tokio::test]
async fn a_malformed_line_gets_a_parse_error_response() {
    let dir = tempfile::tempdir().expect("tempdir");
    let server = fake_server(dir.path(), Vec::new());
    let output = drive(&server, &["not json"]).await;
    let parsed = messages(&output);
    let response = parsed.last().expect("response");
    assert_eq!(response["error"]["code"], -32603);
    assert!(response["error"]["message"]
        .as_str()
        .expect("message")
        .contains("malformed"));
}

#[tokio::test]
async fn a_prompt_for_an_unknown_session_errors() {
    let dir = tempfile::tempdir().expect("tempdir");
    let server = fake_server(dir.path(), Vec::new());
    let output = drive(
        &server,
        &[r#"{"jsonrpc":"2.0","id":5,"method":"session/prompt","params":{"sessionId":"nope","prompt":[{"type":"text","text":"hi"}]}}"#],
    )
    .await;
    let parsed = messages(&output);
    let response = parsed.last().expect("response");
    assert_eq!(response["id"], 5);
    assert!(response["error"]["message"]
        .as_str()
        .expect("message")
        .contains("unknown session"));
}

const ALLOW_ONCE: &str = r#"{"jsonrpc":"2.0","id":1,"result":{"outcome":{"outcome":"selected","optionId":"allow_once"}}}"#;
const REJECT_ONCE: &str = r#"{"jsonrpc":"2.0","id":1,"result":{"outcome":{"outcome":"selected","optionId":"reject_once"}}}"#;

#[tokio::test]
async fn a_permission_round_trip_allows_the_tool() {
    let dir = tempfile::tempdir().expect("tempdir");
    let script = vec![vec![Action::CallTool {
        call_id: "c1".to_string(),
        name: "write_file".to_string(),
        args: r#"{"path": "new.txt", "content": "hi"}"#.to_string(),
    }]];
    let server = fake_server(dir.path(), script);
    let output = drive(
        &server,
        &[
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
            r#"{"jsonrpc":"2.0","id":2,"method":"session/new","params":{}}"#,
        ],
    )
    .await;
    let session_id = session_id_of(&output);

    let prompt = format!(
        r#"{{"jsonrpc":"2.0","id":3,"method":"session/prompt","params":{{"sessionId":"{session_id}","prompt":[{{"type":"text","text":"hi"}}]}}}}"#
    );
    let output = drive(&server, &[&prompt, ALLOW_ONCE]).await;
    let parsed = messages(&output);
    assert!(parsed.iter().any(|message| {
        message["method"] == "session/request_permission"
            && message["params"]["toolCall"]["title"] == "write_file"
    }));
    assert!(parsed
        .iter()
        .any(|message| message["params"]["update"]["toolCallUpdate"]["status"] == "completed"));
    let response = parsed.last().expect("response");
    assert_eq!(response["result"]["stopReason"], "end_turn");
    assert_eq!(
        std::fs::read_to_string(dir.path().join("new.txt")).expect("file"),
        "hi"
    );
}

#[tokio::test]
async fn a_denied_permission_shows_the_error() {
    let dir = tempfile::tempdir().expect("tempdir");
    let script = vec![vec![Action::CallTool {
        call_id: "c1".to_string(),
        name: "write_file".to_string(),
        args: r#"{"path": "new.txt", "content": "hi"}"#.to_string(),
    }]];
    let server = fake_server(dir.path(), script);
    let output = drive(
        &server,
        &[
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
            r#"{"jsonrpc":"2.0","id":2,"method":"session/new","params":{}}"#,
        ],
    )
    .await;
    let session_id = session_id_of(&output);

    let prompt = format!(
        r#"{{"jsonrpc":"2.0","id":3,"method":"session/prompt","params":{{"sessionId":"{session_id}","prompt":[{{"type":"text","text":"hi"}}]}}}}"#
    );
    let output = drive(&server, &[&prompt, REJECT_ONCE]).await;
    let parsed = messages(&output);
    assert!(parsed
        .iter()
        .any(
            |message| message["params"]["update"]["agentMessageChunk"]["content"]["text"]
                == "[error] approval denied: write_file"
        ));
    assert!(!parsed
        .iter()
        .any(|message| message["params"]["update"]["toolCallUpdate"]["status"] == "completed"));
    assert!(!dir.path().join("new.txt").exists());
}

#[tokio::test]
async fn fs_work_flows_through_the_workspace() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("a.txt"), "the body").expect("write");
    let script = vec![vec![Action::CallTool {
        call_id: "c1".to_string(),
        name: "read_file".to_string(),
        args: r#"{"path": "a.txt"}"#.to_string(),
    }]];
    let server = fake_server(dir.path(), script);
    let output = drive(
        &server,
        &[
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
            r#"{"jsonrpc":"2.0","id":2,"method":"session/new","params":{}}"#,
        ],
    )
    .await;
    let session_id = session_id_of(&output);

    let prompt = format!(
        r#"{{"jsonrpc":"2.0","id":3,"method":"session/prompt","params":{{"sessionId":"{session_id}","prompt":[{{"type":"text","text":"hi"}}]}}}}"#
    );
    let output = drive(&server, &[&prompt]).await;
    let parsed = messages(&output);
    // One fs path: the workspace served the read; no permission round-trip
    // (read_file allows) and no client fs callback was involved.
    assert!(!parsed
        .iter()
        .any(|message| message["method"] == "session/request_permission"));
    let completed = parsed
        .iter()
        .find(|message| message["params"]["update"]["toolCallUpdate"]["status"] == "completed")
        .expect("completed tool call");
    assert!(completed["params"]["update"]["toolCallUpdate"]["rawOutput"]
        .as_str()
        .expect("output")
        .contains("the body"));
}

#[tokio::test]
async fn client_fs_methods_are_unhandled() {
    let dir = tempfile::tempdir().expect("tempdir");
    let server = fake_server(dir.path(), Vec::new());
    let output = drive(
        &server,
        &[r#"{"jsonrpc":"2.0","id":9,"method":"fs/read_text_file","params":{"sessionId":"s","path":"a"}}"#],
    )
    .await;
    let parsed = messages(&output);
    let response = parsed.last().expect("response");
    assert_eq!(response["error"]["code"], -32601);
}
