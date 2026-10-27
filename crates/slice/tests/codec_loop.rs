//! Integration: codec selection reaches the loop.
//! Text providers are decoded; native providers bypass decode; malformed
//! output produces a reprompt and recovers.

use std::sync::Mutex;

use futures::executor::block_on;
use orexis_adapters_codec::{JsonCodec, NativeCodec, XmlManualCodec};
use orexis_adapters_fake::{InMemorySession, RecordingSink};
use orexis_core::{Action, Message, ProviderResponse, SessionEvent, Thinking, ToolSpec};
use orexis_ports::{Codec, PortResult, Provider, SessionRead};
use orexis_usecases::{run_turn, ContextConfig, TurnDeps};

struct TextProvider {
    outputs: Mutex<Vec<String>>,
}

impl TextProvider {
    fn new(outputs: Vec<&str>) -> Self {
        Self {
            outputs: Mutex::new(outputs.into_iter().map(str::to_string).collect()),
        }
    }
}

#[async_trait::async_trait]
impl Provider for TextProvider {
    async fn complete(
        &self,
        _messages: &[Message],
        _tools: &[ToolSpec],
        _step: usize,
        _thinking: Thinking,
    ) -> PortResult<ProviderResponse> {
        let text = self.outputs.lock().expect("outputs").remove(0);
        Ok(ProviderResponse {
            actions: vec![Action::Say { text: text.clone() }],
            raw_text: text,
        })
    }
}

struct NativeProvider;

#[async_trait::async_trait]
impl Provider for NativeProvider {
    async fn complete(
        &self,
        _messages: &[Message],
        _tools: &[ToolSpec],
        _step: usize,
        _thinking: Thinking,
    ) -> PortResult<ProviderResponse> {
        Ok(ProviderResponse {
            actions: vec![Action::Done {
                summary: "native".to_string(),
            }],
            raw_text: "ignored by the native codec".to_string(),
        })
    }
}

fn run(provider: &dyn Provider, codec: &dyn Codec, max_steps: usize) -> (bool, Vec<SessionEvent>) {
    let session = InMemorySession::default();
    let sink = RecordingSink::default();
    let outcome = block_on(run_turn(TurnDeps {
        provider,
        tools: &orexis_adapters_fake::EchoTools,
        session: &session,
        reader: &session,
        sink: &sink,
        session_id: "codec-1",
        prompt: "go",
        max_steps,
        codec,
        checkpointer: None,
        verifier: None,
        skip_verify: false,
        thinking: Thinking::Auto,
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
    (outcome.done, events)
}

#[test]
fn text_provider_is_decoded_by_xml_manual() {
    let provider = TextProvider::new(vec![
        "<function name=\"read\"><param name=\"path\">x</param></function><function name=\"finish\"><param name=\"summary\">done</param></function>",
    ]);
    let (done, events) = run(&provider, &XmlManualCodec, 4);
    assert!(done);
    assert!(events.iter().any(
        |event| matches!(event, SessionEvent::ToolCall { call_id, .. } if call_id == "call-0-0")
    ));
    assert!(events.iter().any(
        |event| matches!(event, SessionEvent::Observation { call_id, .. } if call_id == "call-0-0")
    ));
    assert!(events
        .iter()
        .any(|event| matches!(event, SessionEvent::Done { .. })));
}

#[test]
fn native_provider_bypasses_decode() {
    let (done, events) = run(&NativeProvider, &NativeCodec, 2);
    assert!(done);
    assert!(events
        .iter()
        .any(|event| matches!(event, SessionEvent::Done { summary } if summary == "native")));
    assert!(!events
        .iter()
        .any(|event| matches!(event, SessionEvent::Reprompt { .. })));
}

#[test]
fn json_provider_is_decoded() {
    let provider = TextProvider::new(vec![r#"{"action":"done","summary":"json done"}"#]);
    let (done, events) = run(&provider, &JsonCodec, 2);
    assert!(done);
    assert!(events
        .iter()
        .any(|event| matches!(event, SessionEvent::Done { summary } if summary == "json done")));
}

#[test]
fn malformed_text_reprompts_then_recovers() {
    let provider = TextProvider::new(vec![
        "<function name=\"read\">",
        "<function name=\"finish\"><param name=\"summary\">recovered</param></function>",
    ]);
    let (done, events) = run(&provider, &XmlManualCodec, 4);
    assert!(done);
    assert!(events
        .iter()
        .any(|event| matches!(event, SessionEvent::Reprompt { .. })));
    assert!(events
        .iter()
        .any(|event| matches!(event, SessionEvent::Done { summary } if summary == "recovered")));
}
