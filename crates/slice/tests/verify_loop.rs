//! Integration: verify-on-stop refuses `Done` until the command passes.

use std::sync::Mutex;

use futures::executor::block_on;
use orexis_adapters_codec::NativeCodec;
use orexis_adapters_fake::{InMemorySession, RecordingSink, ScriptedProvider};
use orexis_core::{Action, SessionEvent};
use orexis_ports::{PortResult, SessionRead, Verifier, VerifyResult};
use orexis_usecases::{run_turn, ContextConfig, TurnDeps};

struct ScriptedVerifier {
    results: Mutex<Vec<bool>>,
}

impl ScriptedVerifier {
    fn new(results: Vec<bool>) -> Self {
        Self {
            results: Mutex::new(results),
        }
    }
}

#[async_trait::async_trait]
impl Verifier for ScriptedVerifier {
    async fn verify(&self) -> PortResult<VerifyResult> {
        let passed = self.results.lock().expect("results").remove(0);
        Ok(VerifyResult {
            command: "test".to_string(),
            passed,
            output: if passed {
                "ok".to_string()
            } else {
                "test failed".to_string()
            },
        })
    }
}

fn run(verifier: Option<&dyn Verifier>, skip_verify: bool) -> (bool, usize, Vec<SessionEvent>) {
    let session = InMemorySession::default();
    let sink = RecordingSink::default();
    let provider = ScriptedProvider::new(vec![
        vec![Action::Done {
            summary: "first".to_string(),
        }],
        vec![Action::Done {
            summary: "second".to_string(),
        }],
    ]);
    let outcome = block_on(run_turn(TurnDeps {
        provider: &provider,
        tools: &orexis_adapters_fake::EchoTools,
        session: &session,
        reader: &session,
        sink: &sink,
        session_id: "verify-1",
        prompt: "go",
        max_steps: 4,
        codec: &NativeCodec,
        checkpointer: None,
        verifier,
        skip_verify,
        delta_sink: &orexis_ports::NullDeltaSink,
        context: ContextConfig {
            system_prompt: "",
            tools: &[],
            context_files: &[],
            context_limit: 32768,
        },
    }))
    .expect("turn");
    let events = block_on(session.events())
        .expect("events")
        .into_iter()
        .map(|stored| stored.event)
        .collect();
    (outcome.done, outcome.steps, events)
}

#[test]
fn failing_verify_blocks_done_then_passes_on_retry() {
    let verifier = ScriptedVerifier::new(vec![false, true]);
    let (done, steps, events) = run(Some(&verifier), false);
    assert!(done);
    assert_eq!(steps, 2);
    let results: Vec<bool> = events
        .iter()
        .filter_map(|event| match event {
            SessionEvent::Verify { passed, .. } => Some(*passed),
            _ => None,
        })
        .collect();
    assert_eq!(results, vec![false, true]);
    assert!(events.iter().any(|event| matches!(
        event,
        SessionEvent::Done { summary } if summary == "second"
    )));
}

#[test]
fn skip_verify_allows_done_without_running_the_command() {
    let verifier = ScriptedVerifier::new(vec![false]);
    let (done, _steps, events) = run(Some(&verifier), true);
    assert!(done);
    assert!(!events
        .iter()
        .any(|event| matches!(event, SessionEvent::Verify { .. })));
}
