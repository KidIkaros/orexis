//! Integration: the real SQLite store drives the turn loop, then replays.

use futures::executor::block_on;
use orexis_adapters_codec::NativeCodec;
use orexis_adapters_fake::{EchoTools, RecordingSink, ScriptedProvider};
use orexis_adapters_session_sqlite::SqliteSessionStore;
use orexis_core::{Action, SessionEvent};
use orexis_ports::SessionRead;
use orexis_usecases::{run_turn, verify_replay, ContextConfig, TurnDeps};

#[test]
fn sqlite_store_replays_the_turn() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("s.db");
    let store = SqliteSessionStore::open(&path).expect("open");

    let provider = ScriptedProvider::new(vec![
        vec![Action::CallTool {
            call_id: "call-1".to_string(),
            name: "read".to_string(),
            args: "x".to_string(),
        }],
        vec![Action::Done {
            summary: "done".to_string(),
        }],
    ]);
    let tools = EchoTools;
    let sink = RecordingSink::default();

    let outcome = block_on(run_turn(TurnDeps {
        provider: &provider,
        tools: &tools,
        session: &store,
        reader: &store,
        sink: &sink,
        session_id: store.session_id(),
        prompt: "go",
        max_steps: 4,
        codec: &NativeCodec,
        checkpointer: None,
        verifier: None,
        skip_verify: false,
        context: ContextConfig {
            system_prompt: "",
            tools: &[],
            context_files: &[],
            context_limit: 32768,
        },
    }))
    .expect("turn");

    assert!(outcome.done);
    let events: Vec<SessionEvent> = block_on(store.events())
        .expect("events")
        .into_iter()
        .map(|stored| stored.event)
        .collect();
    assert!(verify_replay(&events));

    let id = store.session_id().to_string();
    drop(store);

    let reopened = SqliteSessionStore::open_session(&path, &id).expect("reopen");
    let replay: Vec<SessionEvent> = block_on(reopened.events())
        .expect("replay events")
        .into_iter()
        .map(|stored| stored.event)
        .collect();

    assert_eq!(events, replay);
    assert!(verify_replay(&replay));
}
