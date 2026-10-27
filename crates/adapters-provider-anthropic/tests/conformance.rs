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

use orexis_adapters_provider_anthropic::AnthropicProvider;
use orexis_adapters_provider_catalog::ProviderProfile;
use orexis_core::{Message, Role, Thinking, ToolSpec};

/// The shipped Anthropic profile: `cache.breakpoints: true` (dialect
/// `anthropic`), `thinking: { supported: true, param: thinking }`.
fn profile() -> ProviderProfile {
    serde_yaml::from_str(include_str!(
        "../../adapters-provider-catalog/providers/anthropic.yaml"
    ))
    .expect("profile")
}

fn provider() -> AnthropicProvider<orexis_adapters_http::MockTransport> {
    AnthropicProvider::new(
        profile(),
        "claude-sonnet-4-5",
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
    let body = provider().request_body(&canonical_messages(), &canonical_tools(), Thinking::Auto);
    // System messages are extracted to the `system` field, so the
    // message array starts at the user turn.
    assert_eq!(
        body["tools"][0]["name"], "read_file",
        "tools carry the input_schema shape"
    );
    assert_eq!(
        body["messages"][1]["content"][0]["type"], "tool_use",
        "assistant tool calls are tool_use blocks"
    );
    assert_eq!(
        body["messages"][2]["content"][0]["type"], "tool_result",
        "tool results are tool_result blocks"
    );
}

#[test]
fn cache_breakpoints_are_expressed_on_tools() {
    let body = provider().request_body(&canonical_messages(), &canonical_tools(), Thinking::Auto);
    assert_eq!(
        body["tools"][0]["cache_control"]["type"], "ephemeral",
        "the last tool carries the cache breakpoint"
    );
}

#[test]
fn thinking_config_is_expressed() {
    // The shipped profile declares `thinking: { supported: true, param:
    // thinking, levels: [low, medium, high] }`. The anthropic wire takes
    // an enabled flag plus a token budget; the adapter scales the budget
    // with the declared level's position (1024 << (index + 1)), and
    // Off/Auto omit the parameter — anthropic's default is off.
    let tools = canonical_tools();
    let messages = canonical_messages();
    let provider = provider();
    let on = provider.request_body(&messages, &tools, Thinking::Level(2));
    assert_eq!(on["thinking"]["type"], "enabled");
    assert_eq!(on["thinking"]["budget_tokens"], 8192, "1024 << (2 + 1)");
    assert!(
        on["max_tokens"].as_u64().expect("max_tokens") > 8192,
        "max_tokens must exceed the thinking budget"
    );
    assert!(
        provider
            .request_body(&messages, &tools, Thinking::Off)
            .get("thinking")
            .is_none(),
        "Off omits the parameter: anthropic's default is off"
    );
    assert!(
        provider
            .request_body(&messages, &tools, Thinking::Auto)
            .get("thinking")
            .is_none(),
        "Auto omits the parameter"
    );
    assert_eq!(
        provider.request_body(&messages, &tools, Thinking::On)["thinking"]["budget_tokens"],
        8192,
        "On is the strongest declared level"
    );
}

#[test]
fn system_prefix_carries_the_cache_breakpoint() {
    // The stable prefix — system prompt, tool docs, project context — is
    // assembled as one system message (invariant 3: the prefix is stable
    // within a session; only compaction breaks it), so that is where a
    // breakpoint earns its keep. The conversation tail gets none: it
    // changes every turn, and a moving breakpoint's write cost needs
    // measured evidence first. Two of the wire's four breakpoints are
    // used (system + last tool).
    let body = provider().request_body(&canonical_messages(), &canonical_tools(), Thinking::Auto);
    let system = body["system"].as_array().expect("system blocks");
    let last = system.last().expect("system block");
    assert_eq!(last["cache_control"]["type"], "ephemeral");
}

#[test]
fn breakpoints_off_leave_the_system_a_plain_string() {
    // openrouter-anthropic declares `breakpoints: false` (cache dialect
    // none): the wire form must not change for profiles that ask for
    // nothing.
    let profile: ProviderProfile = serde_yaml::from_str(include_str!(
        "../../adapters-provider-catalog/providers/openrouter-anthropic.yaml"
    ))
    .expect("profile");
    let provider = AnthropicProvider::new(
        profile,
        "anthropic/claude-haiku-5.5",
        Some("test-key".to_string()),
        orexis_adapters_http::MockTransport::new(Vec::new()),
    );
    let body = provider.request_body(&canonical_messages(), &canonical_tools(), Thinking::Auto);
    assert!(body["system"].is_string(), "no breakpoint, no block array");
    assert!(body["tools"]
        .as_array()
        .expect("tools")
        .iter()
        .all(|tool| tool.get("cache_control").is_none()));
}
