//! Local trace export: project a session's append-only `StoredEvent` log
//! into Chrome Trace Event Format (`ui.perfetto.dev` / `chrome://tracing`)
//! with zero external network calls (AGENTS.md: no telemetry).

pub use orexis_core::TraceSpan;
use orexis_core::{render_chrome_trace, StoredEvent};

pub fn export_chrome_trace(session_id: &str, events: &[StoredEvent]) -> String {
    render_chrome_trace(session_id, events)
}

#[cfg(test)]
mod tests {
    use super::*;
    use orexis_core::{Action, SessionEvent, TokenUsage};

    fn stored(seq: u64, event: SessionEvent) -> StoredEvent {
        StoredEvent {
            seq,
            event_id: format!("ev-{seq}"),
            generation_id: 1,
            event,
        }
    }

    #[test]
    fn exports_llm_and_tool_spans_in_chrome_trace_format() {
        let events = vec![
            stored(
                1,
                SessionEvent::SessionStart {
                    session_id: "s-1".into(),
                    format_version: 1,
                },
            ),
            stored(
                2,
                SessionEvent::UserMessage {
                    text: "fix bug".into(),
                },
            ),
            stored(
                3,
                SessionEvent::ModelRequest {
                    profile_id: "local".into(),
                    request_hash: "abc".into(),
                },
            ),
            stored(
                4,
                SessionEvent::ModelResponse {
                    text: "reading".into(),
                    actions: vec![Action::CallTool {
                        call_id: "c1".into(),
                        name: "read_file".into(),
                        args: r#"{"path":"a.rs"}"#.into(),
                    }],
                    usage: Some(TokenUsage {
                        prompt: 120,
                        completion: 30,
                    }),
                },
            ),
            stored(
                5,
                SessionEvent::ToolCall {
                    call_id: "c1".into(),
                    name: "read_file".into(),
                    args: r#"{"path":"a.rs"}"#.into(),
                },
            ),
            stored(
                6,
                SessionEvent::Observation {
                    call_id: "c1".into(),
                    name: "read_file".into(),
                    output: "fn main() {}".into(),
                },
            ),
            stored(
                7,
                SessionEvent::Done {
                    summary: "fixed".into(),
                },
            ),
        ];

        let json_text = export_chrome_trace("s-1", &events);
        let parsed: serde_json::Value = serde_json::from_str(&json_text).expect("valid trace json");
        let trace_events = parsed["traceEvents"].as_array().expect("traceEvents array");
        assert!(trace_events.iter().any(|span| {
            span["cat"] == "llm"
                && span["ph"] == "X"
                && span["args"]["prompt_tokens"] == 120
                && span["args"]["completion_tokens"] == 30
        }));
        assert!(trace_events.iter().any(|span| {
            span["cat"] == "tool"
                && span["ph"] == "X"
                && span["name"] == "tool:read_file"
                && span["args"]["call_id"] == "c1"
        }));
    }
}
