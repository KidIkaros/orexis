//! Conformance fixtures: the A2A client against a recorded transcript —
//! an agent card, a `message/send`, a `tasks/get`, and a streamed task.
//! No live peer; the real-peer check is manual (M7-10).

use std::sync::Arc;
use std::time::Duration;

use orexis_adapters_http::{MockResponse, MockTransport};
use orexis_adapters_protocol_a2a::{
    A2aClient, A2aClientConfig, A2aError, MethodNaming, Outcome, TaskState,
};
use serde_json::Value;

const TIMEOUT: Duration = Duration::from_secs(5);

/// A v0.3-style card: a top-level url, no protocol version.
const CARD_V03: &str = r#"{
  "name": "researcher",
  "description": "an agent that researches things",
  "url": "https://agent.example.com/a2a",
  "version": "0.3.0",
  "capabilities": { "streaming": false, "pushNotifications": false },
  "skills": [
    { "id": "academic-research", "name": "Academic Research", "description": "research with citations" }
  ]
}"#;

/// A v1.0-style card: supportedInterfaces, a protocol version, streaming.
const CARD_V1: &str = r#"{
  "name": "planner",
  "description": "an agent that plans",
  "supportedInterfaces": [
    { "url": "https://planner.example.com/a2a", "protocolBinding": "JSONRPC", "protocolVersion": "1.0" }
  ],
  "capabilities": { "streaming": true, "pushNotifications": true },
  "skills": [
    { "id": "make-plan", "name": "Make a plan", "description": "turns a goal into steps" }
  ]
}"#;

fn config() -> A2aClientConfig {
    A2aClientConfig::new("researcher", "https://agent.example.com")
}

fn client(responses: Vec<MockResponse>) -> (A2aClient, Arc<MockTransport>) {
    let transport = Arc::new(MockTransport::new(Vec::new()).with_responses(responses));
    let client = A2aClient::new(config(), transport.clone(), TIMEOUT);
    (client, transport)
}

/// The methods the client posted, in order. The card fetch is a GET, so
/// it has no JSON-RPC method and is skipped.
fn posted_methods(transport: &MockTransport) -> Vec<String> {
    transport
        .calls
        .lock()
        .expect("calls")
        .iter()
        .filter_map(|(_, _, body)| {
            serde_json::from_str::<serde_json::Value>(body)
                .ok()
                .and_then(|value| {
                    value
                        .get("method")
                        .and_then(|method| method.as_str())
                        .map(str::to_string)
                })
        })
        .collect()
}

#[tokio::test]
async fn the_card_is_fetched_and_validated() {
    let (mut client, transport) = client(vec![MockResponse::json(CARD_V03)]);
    let card = client.discover().await.expect("discover");
    assert_eq!(card.name, "researcher");
    assert_eq!(card.url, "https://agent.example.com/a2a");
    assert_eq!(card.skills.len(), 1);
    assert_eq!(card.skills[0].id, "academic-research");
    assert!(!card.streaming);

    let calls = transport.calls.lock().expect("calls");
    assert_eq!(
        calls[0].1,
        "https://agent.example.com/.well-known/agent-card.json"
    );
}

#[tokio::test]
async fn a_card_without_a_name_is_rejected() {
    let (mut client, _transport) = client(vec![MockResponse::json(r#"{"url":"https://x/a2a"}"#)]);
    let error = client.discover().await.expect_err("no name");
    assert!(matches!(error, A2aError::Shape(_)), "error: {error}");
}

#[tokio::test]
async fn a_missing_card_is_its_own_error() {
    let (mut client, _transport) = client(vec![MockResponse::json("not found").with_status(404)]);
    let error = client.discover().await.expect_err("no card");
    assert_eq!(error, A2aError::NoCard);
}

#[tokio::test]
async fn the_method_naming_follows_the_cards_protocol_version() {
    // A v0.3 card gets the slash form.
    let (mut client, transport) = client(vec![
        MockResponse::json(CARD_V03),
        MockResponse::json(
            r#"{"jsonrpc":"2.0","id":1,"result":{"task":{"id":"task-1","status":{"state":"TASK_STATE_SUBMITTED"}}}}"#,
        ),
    ]);
    client.discover().await.expect("discover");
    client.send_message("research this").await.expect("send");
    assert_eq!(posted_methods(&transport), vec!["message/send"]);

    // A v1.0 card gets the PascalCase form.
    let transport = Arc::new(MockTransport::new(Vec::new()).with_responses(vec![
        MockResponse::json(CARD_V1),
        MockResponse::json(
            r#"{"jsonrpc":"2.0","id":1,"result":{"task":{"id":"task-1","status":{"state":"TASK_STATE_SUBMITTED"}}}}"#,
        ),
    ]));
    let mut client = A2aClient::new(config(), transport.clone(), TIMEOUT);
    client.discover().await.expect("discover");
    assert_eq!(
        client.card().map(|card| card.naming()),
        Some(MethodNaming::PascalCase)
    );
    client.send_message("plan this").await.expect("send");
    assert_eq!(posted_methods(&transport), vec!["SendMessage"]);
}

#[tokio::test]
async fn sending_a_message_returns_the_task() {
    let (mut client, _transport) = client(vec![
        MockResponse::json(CARD_V03),
        MockResponse::json(
            r#"{"jsonrpc":"2.0","id":1,"result":{"task":{"id":"task-1","contextId":"ctx-1","status":{"state":"TASK_STATE_WORKING"}}}}"#,
        ),
    ]);
    client.discover().await.expect("discover");
    let outcome = client.send_message("research this").await.expect("send");
    match outcome {
        Outcome::Task(task) => {
            assert_eq!(task.id, "task-1");
            assert_eq!(task.context_id.as_deref(), Some("ctx-1"));
            assert_eq!(task.state, TaskState::Working);
            assert!(!task.state.is_terminal());
        }
        other => panic!("expected a task, got {other:?}"),
    }
}

#[tokio::test]
async fn a_direct_message_is_read_as_a_message() {
    let (mut client, _transport) = client(vec![
        MockResponse::json(CARD_V03),
        MockResponse::json(
            r#"{"jsonrpc":"2.0","id":1,"result":{"message":{"role":"ROLE_AGENT","parts":[{"text":"the answer"}]}}}"#,
        ),
    ]);
    client.discover().await.expect("discover");
    let outcome = client.send_message("a question").await.expect("send");
    match outcome {
        Outcome::Message { text } => assert_eq!(text, "the answer"),
        other => panic!("expected a message, got {other:?}"),
    }
}

#[tokio::test]
async fn reading_a_task_back_reports_its_state() {
    let (mut client, transport) = client(vec![
        MockResponse::json(CARD_V03),
        MockResponse::json(
            r#"{"jsonrpc":"2.0","id":1,"result":{"task":{"id":"task-1","status":{"state":"TASK_STATE_COMPLETED","message":{"role":"ROLE_AGENT","parts":[{"text":"done"}]}}}}}"#,
        ),
    ]);
    client.discover().await.expect("discover");
    let outcome = client.get_task("task-1").await.expect("get");
    match outcome {
        Outcome::Task(task) => {
            assert_eq!(task.state, TaskState::Completed);
            assert!(task.state.is_terminal());
            assert_eq!(task.text, "done");
        }
        other => panic!("expected a task, got {other:?}"),
    }
    assert_eq!(posted_methods(&transport), vec!["tasks/get"]);
}

#[tokio::test]
async fn an_unknown_task_state_degrades_visibly() {
    let (mut client, _transport) = client(vec![
        MockResponse::json(CARD_V03),
        MockResponse::json(
            r#"{"jsonrpc":"2.0","id":1,"result":{"task":{"id":"task-1","status":{"state":"TASK_STATE_WAT"}}}}"#,
        ),
    ]);
    client.discover().await.expect("discover");
    let outcome = client.get_task("task-1").await.expect("get");
    match outcome {
        Outcome::Task(task) => {
            assert_eq!(task.state, TaskState::Unknown);
            assert_eq!(task.raw_state, "TASK_STATE_WAT");
            assert!(
                !task.state.is_terminal(),
                "an unknown state must not read as still working"
            );
        }
        other => panic!("expected a task, got {other:?}"),
    }
}

#[tokio::test]
async fn two_messages_of_the_same_length_carry_distinct_message_ids() {
    // A2A peers may treat a repeated messageId as a replay and refuse it,
    // so the id must not be derived from the text alone.
    let (mut client, transport) = client(vec![
        MockResponse::json(CARD_V03),
        MockResponse::json(
            r#"{"jsonrpc":"2.0","id":1,"result":{"message":{"role":"ROLE_AGENT","parts":[{"text":"one"}]}}}"#,
        ),
        MockResponse::json(
            r#"{"jsonrpc":"2.0","id":2,"result":{"message":{"role":"ROLE_AGENT","parts":[{"text":"two"}]}}}"#,
        ),
    ]);
    client.discover().await.expect("discover");
    client.send_message("aaaa").await.expect("first send");
    client.send_message("bbbb").await.expect("second send");
    let ids: Vec<String> = transport
        .calls
        .lock()
        .expect("calls")
        .iter()
        .filter_map(|(_, _, body)| {
            serde_json::from_str::<serde_json::Value>(body)
                .ok()?
                .pointer("/params/message/messageId")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .collect();
    assert_eq!(ids.len(), 2, "both sends recorded: {ids:?}");
    assert_ne!(ids[0], ids[1], "message ids collided: {ids:?}");
}

#[tokio::test]
async fn a_peer_error_is_an_error() {
    let (mut client, _transport) = client(vec![
        MockResponse::json(CARD_V03),
        MockResponse::json(
            r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32001,"message":"Task not found"}}"#,
        ),
    ]);
    client.discover().await.expect("discover");
    let error = client.get_task("missing").await.expect_err("an error");
    assert!(matches!(error, A2aError::Shape(_)), "error: {error}");
}

#[tokio::test]
async fn streaming_is_refused_when_the_card_does_not_advertise_it() {
    let (mut client, _transport) = client(vec![MockResponse::json(CARD_V03)]);
    client.discover().await.expect("discover");
    let error = client
        .stream_message("research this")
        .await
        .expect_err("no streaming");
    assert!(matches!(error, A2aError::Unsupported(_)), "error: {error}");
}

#[tokio::test]
async fn a_streamed_task_is_read_frame_by_frame() {
    let (mut client, transport) = client(vec![
        MockResponse::json(CARD_V1),
        MockResponse::sse(
            concat!(
                "event: message\ndata: {\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"task\":{\"id\":\"task-9\",\"status\":{\"state\":\"TASK_STATE_SUBMITTED\"}}}}\n\n",
                "event: message\ndata: {\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"task\":{\"id\":\"task-9\",\"status\":{\"state\":\"TASK_STATE_COMPLETED\"}}}}\n\n"
            )
            .to_string(),
        ),
    ]);
    client.discover().await.expect("discover");
    let outcomes = client.stream_message("plan this").await.expect("stream");
    assert_eq!(outcomes.len(), 2);
    assert_eq!(posted_methods(&transport), vec!["SendStreamingMessage"]);
    match &outcomes[1] {
        Outcome::Task(task) => assert_eq!(task.state, TaskState::Completed),
        other => panic!("expected a task, got {other:?}"),
    }
}
