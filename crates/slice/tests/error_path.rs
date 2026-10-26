//! Integration: a tool failure is persisted as an `Error` event and returned.

use futures::executor::block_on;
use orexis_adapters_codec::NativeCodec;
use orexis_adapters_fake::{RecordingSink, ScriptedProvider};
use orexis_adapters_session_sqlite::SqliteSessionStore;
use orexis_core::{Action, Observation, SessionEvent};
use orexis_ports::{PortError, PortResult, SessionRead, ToolExecutor};
use orexis_usecases::{run_turn, ContextConfig, TurnDeps};

struct FailingTools;

#[async_trait::async_trait]
impl ToolExecutor for FailingTools {
    async fn execute(&self, _name: &str, _args: &str) -> PortResult<Observation> {
        Err(PortError("tool boom".to_string()))
    }
}

#[test]
fn tool_failure_is_persisted_and_returned() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = SqliteSessionStore::open(&dir.path().join("s.db")).expect("open");
    let provider = ScriptedProvider::new(vec![vec![Action::CallTool {
        call_id: "call-1".to_string(),
        name: "read".to_string(),
        args: "x".to_string(),
    }]]);
    let tools = FailingTools;
    let sink = RecordingSink::default();

    let result = block_on(run_turn(TurnDeps {
        provider: &provider,
        tools: &tools,
        session: &store,
        reader: &store,
        sink: &sink,
        session_id: store.session_id(),
        prompt: "go",
        max_steps: 2,
        codec: &NativeCodec,
        context: ContextConfig {
            system_prompt: "",
            tools: &[],
            context_files: &[],
            context_limit: 32768,
        },
    }));

    assert!(result.is_err());
    let events = block_on(store.events()).expect("events");
    assert!(events
        .iter()
        .any(|stored| matches!(stored.event, SessionEvent::Error { .. })));
}
