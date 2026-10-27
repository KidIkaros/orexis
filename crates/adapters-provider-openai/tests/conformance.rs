//! Per-dialect provider conformance fixtures (issue 013, ADR-0018).
//!
//! A fixture is one fidelity capability — a canonical set of model-visible
//! messages plus the exact wire body the adapter must produce — run against
//! the shipped profile's declarations, sans-io and in CI. Three states:
//!
//! - **expressed**: the adapter's wire carries the capability; the fixture
//!   asserts the recorded body.
//! - **gap (expressible-but-unimplemented)**: the wire can carry it and the
//!   shipped profile declares it, but the adapter does not emit it. The
//!   ratchet asserts *absence*: closing the gap turns the test red and
//!   forces a real fixture plus a manifest flip. This is an adapter fix,
//!   never a native-adapter trigger (ADR-0016 trigger 2 is for
//!   wire-inexpressible capabilities).
//! - **wire-inexpressible**: the compatible wire cannot carry it at all —
//!   the ADR-0016 fidelity trigger; none is known today.

use orexis_adapters_provider_catalog::ProviderProfile;
use orexis_adapters_provider_openai::OpenAiCompatibleProvider;
use orexis_core::{Message, Role, ToolSpec};

/// The shipped OpenAI profile: `cache.breakpoints: true` (dialect
/// `open_ai`), `thinking: { supported: true, param: reasoning_effort }`.
fn profile() -> ProviderProfile {
    serde_yaml::from_str(include_str!(
        "../../adapters-provider-catalog/providers/openai.yaml"
    ))
    .expect("profile")
}

fn provider() -> OpenAiCompatibleProvider<orexis_adapters_http::MockTransport> {
    OpenAiCompatibleProvider::new(
        profile(),
        "gpt-5",
        Some("test-key".to_string()),
        orexis_adapters_http::MockTransport::new(Vec::new()),
    )
}

/// The canonical conversation: system, user, an assistant tool call, and
/// its tool result — the full shape every capability fixture starts from.
fn canonical_messages() -> Vec<Message> {
    vec![
        Message {
            role: Role::System,
            text: "be a coding agent".to_string(),
            tool_call_id: None,
            tool_calls: Vec::new(),
        },
        Message {
            role: Role::User,
            text: "read the file".to_string(),
            tool_call_id: None,
            tool_calls: Vec::new(),
        },
        Message {
            role: Role::Assistant,
            text: String::new(),
            tool_call_id: None,
            tool_calls: vec![orexis_core::ToolCall {
                call_id: "call-1".to_string(),
                name: "read_file".to_string(),
                args: r#"{"path":"a.txt"}"#.to_string(),
            }],
        },
        Message {
            role: Role::Tool,
            text: "the contents".to_string(),
            tool_call_id: Some("call-1".to_string()),
            tool_calls: Vec::new(),
        },
    ]
}

fn canonical_tools() -> Vec<ToolSpec> {
    vec![ToolSpec {
        name: "read_file".to_string(),
        description: "Read a file".to_string(),
        schema: serde_json::json!({"type": "object"}),
    }]
}

#[test]
fn tool_call_shape_is_expressed() {
    let body = provider().request_body(&canonical_messages(), &canonical_tools());
    assert_eq!(
        body["tools"][0]["function"]["name"], "read_file",
        "tools carry the function shape"
    );
    assert_eq!(
        body["messages"][2]["tool_calls"][0]["id"], "call-1",
        "assistant tool calls carry the wire shape"
    );
    assert_eq!(
        body["messages"][3]["tool_call_id"], "call-1",
        "tool results pair by call id"
    );
}

#[test]
fn cache_is_automatic_on_the_open_ai_dialect() {
    // The open_ai dialect has no explicit cache breakpoints: caching is
    // automatic endpoint-side, so `breakpoints: true` asks for nothing on
    // the wire. This is a no-op pass, not a gap — recorded so a future
    // dialect change (e.g. an explicit-cache compatible endpoint) has a
    // fixture to extend rather than a silent assumption.
    let body = provider().request_body(&canonical_messages(), &canonical_tools());
    assert!(
        body.get("cache_control").is_none()
            && body["messages"]
                .as_array()
                .expect("messages")
                .iter()
                .all(|message| message.get("cache_control").is_none()),
        "the open_ai wire carries no explicit cache fields"
    );
}

#[test]
fn thinking_config_is_a_recorded_gap() {
    // The shipped profile declares `thinking: { supported: true, param:
    // reasoning_effort }`; the wire carries the parameter; the adapter
    // never emits it. Ratchet: this asserts ABSENCE — closing the gap
    // turns this test red, forcing a real fixture here and the manifest
    // flip. Follow-up ticket 017.
    let body = provider().request_body(&canonical_messages(), &canonical_tools());
    assert!(
        body.get("reasoning_effort").is_none(),
        "the gap is closed — write the real fixture and flip the manifest"
    );
}
