//! Pure domain types for the vertical slice. No I/O, no ports, no runtime.

mod entity;
mod evolution;
mod team;
pub use entity::*;
pub use evolution::*;
pub use team::*;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    User,
    Assistant,
    Tool,
    System,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub text: String,
    #[serde(default)]
    pub tool_call_id: Option<String>,
    #[serde(default)]
    pub tool_calls: Vec<ToolCall>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Action {
    Say {
        text: String,
    },
    CallTool {
        call_id: String,
        name: String,
        args: String,
    },
    Done {
        summary: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolCall {
    pub call_id: String,
    pub name: String,
    pub args: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Observation {
    pub name: String,
    pub output: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderResponse {
    pub actions: Vec<Action>,
    pub raw_text: String,
    /// Tokens the provider reports for this call, when it reports them.
    pub usage: Option<TokenUsage>,
}

/// Token accounting for one provider call (§10 bench metrics).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt: u64,
    pub completion: u64,
}

impl TokenUsage {
    pub fn total(&self) -> u64 {
        self.prompt.saturating_add(self.completion)
    }

    pub fn accumulate(&mut self, other: &Self) {
        self.prompt = self.prompt.saturating_add(other.prompt);
        self.completion = self.completion.saturating_add(other.completion);
    }
}

/// Per-million-token pricing in microdollars (1 USD = 1_000_000 microdollars),
/// keeping cost accounting exact and integer-only in core.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ModelPricing {
    pub prompt_microdollars_per_million: u64,
    pub completion_microdollars_per_million: u64,
}

impl ModelPricing {
    pub fn cost_microdollars(&self, usage: &TokenUsage) -> u64 {
        let prompt = (usage.prompt as u128)
            .saturating_mul(self.prompt_microdollars_per_million as u128)
            / 1_000_000;
        let completion = (usage.completion as u128)
            .saturating_mul(self.completion_microdollars_per_million as u128)
            / 1_000_000;
        prompt.saturating_add(completion).min(u64::MAX as u128) as u64
    }
}

/// Hard ceiling on a turn or session's token and dollar spend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CostBudget {
    pub max_tokens: Option<u64>,
    pub max_cost_microdollars: Option<u64>,
    pub pricing: ModelPricing,
}

impl CostBudget {
    /// Returns the reason string when `usage` exceeds the configured token
    /// or dollar ceiling, or `None` when within budget.
    pub fn check(&self, usage: &TokenUsage) -> Option<String> {
        if let Some(max_tokens) = self.max_tokens {
            let total = usage.total();
            if total > max_tokens {
                return Some(format!(
                    "budget exceeded: {total} tokens > {max_tokens} token limit"
                ));
            }
        }
        if let Some(max_cost) = self.max_cost_microdollars {
            let cost = self.pricing.cost_microdollars(usage);
            if cost > max_cost {
                return Some(format!(
                    "budget exceeded: ${:.4} ({cost} microdollars) > ${:.4} limit",
                    cost as f64 / 1_000_000.0,
                    max_cost as f64 / 1_000_000.0
                ));
            }
        }
        None
    }
}

/// Sum the token usage recorded across a session's event log.
pub fn session_usage(events: &[SessionEvent]) -> TokenUsage {
    let mut total = TokenUsage::default();
    for event in events {
        if let SessionEvent::ModelResponse {
            usage: Some(usage), ..
        } = event
        {
            total.accumulate(usage);
        }
    }
    total
}

/// Extract a scannable target string from a tool argument payload (`path`,
/// `command`, `query`, or `title` when `args` is a JSON object).
pub fn extract_tool_target(args: &str) -> String {
    if let Ok(serde_json::Value::Object(map)) = serde_json::from_str::<serde_json::Value>(args) {
        for key in ["path", "command", "query", "title"] {
            if let Some(value) = map.get(key).and_then(serde_json::Value::as_str) {
                return value.to_string();
            }
        }
    }
    args.to_string()
}

/// One span or instant event in Chrome Trace Event Format (`ui.perfetto.dev`
/// / `chrome://tracing`), projected locally from a session's `StoredEvent`
/// log with zero external network telemetry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceSpan {
    pub name: String,
    pub cat: String,
    pub ph: String,
    pub ts: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dur: Option<u64>,
    pub pid: u32,
    pub tid: u32,
    pub args: serde_json::Value,
}

/// Project a session's `StoredEvent` log into a Chrome Trace Event JSON
/// document (`{"session_id": ..., "traceEvents": [...]}`).
pub fn render_chrome_trace(session_id: &str, events: &[StoredEvent]) -> String {
    let mut spans = Vec::new();
    let mut open_llm: Option<(u64, String, String)> = None;
    let mut open_tools: std::collections::BTreeMap<String, (u64, String, String)> =
        std::collections::BTreeMap::new();

    for stored in events {
        let ts = stored.seq.saturating_mul(1_000);
        match &stored.event {
            SessionEvent::ModelRequest {
                profile_id,
                request_hash,
            } => {
                open_llm = Some((ts, profile_id.clone(), request_hash.clone()));
            }
            SessionEvent::ModelResponse {
                text,
                actions,
                usage,
            } => {
                let (start_ts, profile_id, request_hash) = open_llm.take().unwrap_or((
                    ts.saturating_sub(500),
                    String::new(),
                    String::new(),
                ));
                let dur = ts.saturating_sub(start_ts).max(1);
                let u = usage.unwrap_or_default();
                spans.push(TraceSpan {
                    name: "model_step".to_string(),
                    cat: "llm".to_string(),
                    ph: "X".to_string(),
                    ts: start_ts,
                    dur: Some(dur),
                    pid: 1,
                    tid: stored.generation_id.max(1) as u32,
                    args: serde_json::json!({
                        "seq": stored.seq,
                        "profile_id": profile_id,
                        "request_hash": request_hash,
                        "prompt_tokens": u.prompt,
                        "completion_tokens": u.completion,
                        "total_tokens": u.total(),
                        "actions": actions.len(),
                        "text_bytes": text.len(),
                    }),
                });
            }
            SessionEvent::ToolCall {
                call_id,
                name,
                args,
            } => {
                open_tools.insert(call_id.clone(), (ts, name.clone(), args.clone()));
            }
            SessionEvent::Observation {
                call_id,
                name,
                output,
            } => {
                let (start_ts, tool_name, tool_args) = open_tools.remove(call_id).unwrap_or((
                    ts.saturating_sub(500),
                    name.clone(),
                    String::new(),
                ));
                let dur = ts.saturating_sub(start_ts).max(1);
                spans.push(TraceSpan {
                    name: format!("tool:{tool_name}"),
                    cat: "tool".to_string(),
                    ph: "X".to_string(),
                    ts: start_ts,
                    dur: Some(dur),
                    pid: 1,
                    tid: stored.generation_id.max(1) as u32,
                    args: serde_json::json!({
                        "seq": stored.seq,
                        "call_id": call_id,
                        "args": tool_args,
                        "output_bytes": output.len(),
                    }),
                });
            }
            other => {
                spans.push(TraceSpan {
                    name: other.kind().to_string(),
                    cat: "session".to_string(),
                    ph: "i".to_string(),
                    ts,
                    dur: None,
                    pid: 1,
                    tid: stored.generation_id.max(1) as u32,
                    args: serde_json::json!({
                        "seq": stored.seq,
                        "event_id": stored.event_id,
                    }),
                });
            }
        }
    }

    let doc = serde_json::json!({
        "session_id": session_id,
        "traceEvents": spans,
    });
    serde_json::to_string_pretty(&doc).unwrap_or_else(|_| "{}".to_string())
}

/// A tool the model may call. `schema` is a JSON Schema object.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub schema: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodecId {
    Native,
    XmlManual,
    Json,
    Lenient,
}

impl CodecId {
    pub fn name(self) -> &'static str {
        match self {
            Self::Native => "native",
            Self::XmlManual => "xml-manual",
            Self::Json => "json",
            Self::Lenient => "lenient",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextFile {
    pub path: String,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Decision {
    Allow,
    Ask,
    Deny,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionRule {
    pub tool: String,
    pub decision: Decision,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionPolicy {
    pub plan_mode: bool,
    pub rules: Vec<PermissionRule>,
}

impl PermissionPolicy {
    pub fn standard() -> Self {
        Self {
            plan_mode: false,
            rules: Vec::new(),
        }
    }

    pub fn plan() -> Self {
        Self {
            plan_mode: true,
            rules: Vec::new(),
        }
    }

    pub fn with_rules(mut self, rules: Vec<PermissionRule>) -> Self {
        self.rules = rules;
        self
    }

    /// Explicit rules override defaults; among rules, deny > ask > allow.
    /// Plan mode denies every mutating tool regardless of rules.
    pub fn decide(&self, tool: &str) -> Decision {
        let mut explicit: Option<Decision> = None;
        for rule in &self.rules {
            if rule_matches(&rule.tool, tool) {
                explicit =
                    Some(explicit.map_or(rule.decision, |current| current.max(rule.decision)));
            }
        }
        let decision = explicit.unwrap_or_else(|| self.default_decision(tool));
        if self.plan_mode && is_mutating(tool) {
            Decision::Deny
        } else {
            decision
        }
    }

    fn default_decision(&self, tool: &str) -> Decision {
        match tool {
            "read_file" | "list_dir" | "search" | "git_status" | "git_diff" | "board_list" => {
                Decision::Allow
            }
            "write_file" | "edit_file" | "shell" | "verify" => Decision::Ask,
            "git_commit" => Decision::Deny,
            _ => Decision::Ask,
        }
    }
}

/// A rule names a tool exactly, all tools (`*`), or a namespace prefix
/// (`mcp/fs/*`). A prefix rule matches `mcp/fs/…` but not `mcp/fs-other/…`.
fn rule_matches(rule_tool: &str, tool: &str) -> bool {
    if rule_tool == "*" || rule_tool == tool {
        return true;
    }
    rule_tool
        .strip_suffix("/*")
        .and_then(|prefix| tool.strip_prefix(prefix))
        .is_some_and(|rest| rest.starts_with('/') && rest.len() > 1)
}

/// External tools (namespaced by protocol) reach outside the workspace;
/// plan mode denies them and their output is untrusted content.
pub fn is_external_tool(tool: &str) -> bool {
    tool.starts_with("mcp/") || tool.starts_with("utcp/") || tool.starts_with("a2a/")
}

fn is_mutating(tool: &str) -> bool {
    matches!(
        tool,
        "write_file" | "edit_file" | "shell" | "verify" | "git_commit"
    ) || is_external_tool(tool)
}

#[cfg(test)]
mod policy_tests {
    use super::*;

    #[test]
    fn deny_beats_ask_beats_allow() {
        let policy = PermissionPolicy::standard().with_rules(vec![
            PermissionRule {
                tool: "shell".into(),
                decision: Decision::Allow,
            },
            PermissionRule {
                tool: "shell".into(),
                decision: Decision::Ask,
            },
        ]);
        assert_eq!(policy.decide("shell"), Decision::Ask);

        let policy = policy.with_rules(vec![
            PermissionRule {
                tool: "shell".into(),
                decision: Decision::Allow,
            },
            PermissionRule {
                tool: "shell".into(),
                decision: Decision::Deny,
            },
        ]);
        assert_eq!(policy.decide("shell"), Decision::Deny);
    }

    #[test]
    fn explicit_allow_overrides_the_default_ask() {
        let policy = PermissionPolicy::standard().with_rules(vec![PermissionRule {
            tool: "shell".into(),
            decision: Decision::Allow,
        }]);
        assert_eq!(policy.decide("shell"), Decision::Allow);
    }

    #[test]
    fn plan_mode_denies_mutating_tools() {
        let policy = PermissionPolicy::plan();
        assert_eq!(policy.decide("read_file"), Decision::Allow);
        assert_eq!(policy.decide("write_file"), Decision::Deny);
        assert_eq!(policy.decide("shell"), Decision::Deny);
    }

    #[test]
    fn defaults_are_conservative() {
        let policy = PermissionPolicy::standard();
        assert_eq!(policy.decide("read_file"), Decision::Allow);
        assert_eq!(policy.decide("edit_file"), Decision::Ask);
        assert_eq!(policy.decide("git_commit"), Decision::Deny);
        assert_eq!(policy.decide("unknown_tool"), Decision::Ask);
    }

    #[test]
    fn board_tools_read_free_write_asks() {
        // The board's write surface goes through the same gate as any
        // mutation (M8-11); reads are free like any read tool.
        let policy = PermissionPolicy::standard();
        assert_eq!(policy.decide("board_list"), Decision::Allow);
        assert_eq!(policy.decide("board_add"), Decision::Ask);
    }

    #[test]
    fn external_tools_default_to_ask() {
        let policy = PermissionPolicy::standard();
        assert_eq!(policy.decide("mcp/fs/read_file"), Decision::Ask);
        assert_eq!(policy.decide("utcp/http/call"), Decision::Ask);
    }

    #[test]
    fn prefix_rules_match_namespaced_tools() {
        let policy = PermissionPolicy::standard().with_rules(vec![PermissionRule {
            tool: "mcp/fs/*".into(),
            decision: Decision::Allow,
        }]);
        assert_eq!(policy.decide("mcp/fs/read_file"), Decision::Allow);
        assert_eq!(policy.decide("mcp/fs/sub/x"), Decision::Allow);
        assert_eq!(policy.decide("mcp/other/read_file"), Decision::Ask);
        assert_eq!(policy.decide("mcp/fs"), Decision::Ask);
        assert_eq!(policy.decide("mcp/fs-other/x"), Decision::Ask);
    }

    #[test]
    fn deny_prefix_beats_allow_prefix() {
        let policy = PermissionPolicy::standard().with_rules(vec![
            PermissionRule {
                tool: "mcp/fs/*".into(),
                decision: Decision::Allow,
            },
            PermissionRule {
                tool: "mcp/fs/write_file".into(),
                decision: Decision::Deny,
            },
        ]);
        assert_eq!(policy.decide("mcp/fs/read_file"), Decision::Allow);
        assert_eq!(policy.decide("mcp/fs/write_file"), Decision::Deny);
    }

    #[test]
    fn plan_mode_denies_external_tools_regardless_of_rules() {
        let policy = PermissionPolicy::plan().with_rules(vec![PermissionRule {
            tool: "mcp/fs/*".into(),
            decision: Decision::Allow,
        }]);
        assert_eq!(policy.decide("mcp/fs/read_file"), Decision::Deny);
        assert_eq!(policy.decide("utcp/http/call"), Decision::Deny);
    }
}

pub const FORMAT_VERSION: u32 = 1;

/// A durable event reference returned by an append.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventRef {
    pub seq: u64,
    pub event_id: String,
}

/// An event plus its stored metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredEvent {
    pub seq: u64,
    pub event_id: String,
    pub generation_id: u64,
    pub event: SessionEvent,
}

/// The durable session log (ADR-0001), reduced to the slice's event kinds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SessionEvent {
    SessionStart {
        session_id: String,
        format_version: u32,
    },
    UserMessage {
        text: String,
    },
    ModelRequest {
        profile_id: String,
        request_hash: String,
    },
    ModelResponse {
        text: String,
        actions: Vec<Action>,
        /// Tokens the provider reported for this response (§10 bench;
        /// absent on providers that do not report and on old logs).
        #[serde(default)]
        usage: Option<TokenUsage>,
    },
    ToolCall {
        call_id: String,
        name: String,
        args: String,
    },
    Observation {
        call_id: String,
        name: String,
        output: String,
    },
    Done {
        summary: String,
    },
    Error {
        message: String,
    },
    Reprompt {
        note: String,
    },
    Interrupted {
        reason: String,
    },
    Recovered {
        reason: String,
    },
    ApprovalDecision {
        tool: String,
        args: String,
        approved: bool,
        always: bool,
    },
    Checkpoint {
        id: String,
        label: String,
    },
    Info {
        message: String,
    },
    Verify {
        command: String,
        passed: bool,
    },
    Compaction {
        range_start_seq: u64,
        range_end_seq: u64,
        summary: String,
    },
}

impl SessionEvent {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::SessionStart { .. } => "session_start",
            Self::UserMessage { .. } => "user_message",
            Self::ModelRequest { .. } => "model_request",
            Self::ModelResponse { .. } => "model_response",
            Self::ToolCall { .. } => "tool_call",
            Self::Observation { .. } => "observation",
            Self::Done { .. } => "done",
            Self::Error { .. } => "error",
            Self::Reprompt { .. } => "reprompt",
            Self::Interrupted { .. } => "interrupted",
            Self::Recovered { .. } => "recovered",
            Self::ApprovalDecision { .. } => "approval_decision",
            Self::Checkpoint { .. } => "checkpoint",
            Self::Info { .. } => "info",
            Self::Verify { .. } => "verify",
            Self::Compaction { .. } => "compaction",
        }
    }
}

/// Pure projection: the model-visible messages derived from the log.
pub fn derive_messages(events: &[SessionEvent]) -> Vec<Message> {
    let mut out = Vec::new();
    for event in events {
        match event {
            SessionEvent::UserMessage { text } => out.push(Message {
                role: Role::User,
                text: text.clone(),
                tool_call_id: None,
                tool_calls: Vec::new(),
            }),
            SessionEvent::ModelResponse { text, actions, .. } => {
                let tool_calls: Vec<ToolCall> = actions
                    .iter()
                    .filter_map(|action| match action {
                        Action::CallTool {
                            call_id,
                            name,
                            args,
                        } => Some(ToolCall {
                            call_id: call_id.clone(),
                            name: name.clone(),
                            args: args.clone(),
                        }),
                        _ => None,
                    })
                    .collect();
                if !text.is_empty() || !tool_calls.is_empty() {
                    out.push(Message {
                        role: Role::Assistant,
                        text: text.clone(),
                        tool_call_id: None,
                        tool_calls,
                    });
                }
            }
            SessionEvent::Observation {
                call_id,
                name,
                output,
            } => out.push(Message {
                role: Role::Tool,
                text: format!("{name}: {output}"),
                tool_call_id: Some(call_id.clone()),
                tool_calls: Vec::new(),
            }),
            SessionEvent::Reprompt { note } => out.push(Message {
                role: Role::User,
                text: format!("[reprompt] {note}"),
                tool_call_id: None,
                tool_calls: Vec::new(),
            }),
            SessionEvent::Compaction { summary, .. } => {
                out.clear();
                out.push(Message {
                    role: Role::System,
                    text: format!("[summary] {summary}"),
                    tool_call_id: None,
                    tool_calls: Vec::new(),
                });
            }
            _ => {}
        }
    }
    out
}

/// A stable textual rendering, used to compare a live request with its replay.
pub fn render_messages(messages: &[Message]) -> String {
    messages
        .iter()
        .map(|m| format!("{:?}: {}", m.role, m.text))
        .collect::<Vec<_>>()
        .join("\n")
}

/// A content digest over the model-visible messages.
pub fn hash_messages(messages: &[Message]) -> String {
    let mut hasher = blake3::Hasher::new();
    for message in messages {
        hasher.update(format!("{:?}:{}\n", message.role, message.text).as_bytes());
    }
    hasher.finalize().to_hex().to_string()
}

/// Redact credential-shaped tokens at write time (ADR-0001).
pub fn redact_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for token in text.split_inclusive(char::is_whitespace) {
        let trimmed = token.trim_end();
        if looks_secret(trimmed) {
            let digest = blake3::hash(trimmed.as_bytes()).to_hex().to_string();
            out.push_str(&format!("[REDACTED:{}]", &digest[..8]));
            out.push_str(&token[trimmed.len()..]);
        } else {
            out.push_str(token);
        }
    }
    out
}

fn looks_secret(token: &str) -> bool {
    let candidate = token.trim_matches(|c: char| !c.is_alphanumeric() && c != '-' && c != '_');
    (candidate.starts_with("sk-") && candidate.len() >= 20)
        || (candidate.starts_with("ghp_") && candidate.len() >= 20)
        || (candidate.starts_with("xoxb-") && candidate.len() >= 20)
        || (candidate.starts_with("AKIA") && candidate.len() == 20)
}

pub fn redact_event(event: &SessionEvent) -> SessionEvent {
    match event {
        SessionEvent::SessionStart {
            session_id,
            format_version,
        } => SessionEvent::SessionStart {
            session_id: session_id.clone(),
            format_version: *format_version,
        },
        SessionEvent::UserMessage { text } => SessionEvent::UserMessage {
            text: redact_text(text),
        },
        SessionEvent::ModelRequest {
            profile_id,
            request_hash,
        } => SessionEvent::ModelRequest {
            profile_id: profile_id.clone(),
            request_hash: request_hash.clone(),
        },
        SessionEvent::ModelResponse {
            text,
            actions,
            usage,
        } => SessionEvent::ModelResponse {
            text: redact_text(text),
            actions: actions.iter().map(redact_action).collect(),
            usage: *usage,
        },
        SessionEvent::ToolCall {
            call_id,
            name,
            args,
        } => SessionEvent::ToolCall {
            call_id: call_id.clone(),
            name: name.clone(),
            args: redact_text(args),
        },
        SessionEvent::Observation {
            call_id,
            name,
            output,
        } => SessionEvent::Observation {
            call_id: call_id.clone(),
            name: name.clone(),
            output: redact_text(output),
        },
        SessionEvent::Done { summary } => SessionEvent::Done {
            summary: redact_text(summary),
        },
        SessionEvent::Error { message } => SessionEvent::Error {
            message: redact_text(message),
        },
        SessionEvent::Reprompt { note } => SessionEvent::Reprompt {
            note: redact_text(note),
        },
        SessionEvent::Interrupted { reason } => SessionEvent::Interrupted {
            reason: redact_text(reason),
        },
        SessionEvent::Recovered { reason } => SessionEvent::Recovered {
            reason: redact_text(reason),
        },
        SessionEvent::ApprovalDecision {
            tool,
            args,
            approved,
            always,
        } => SessionEvent::ApprovalDecision {
            tool: tool.clone(),
            args: redact_text(args),
            approved: *approved,
            always: *always,
        },
        SessionEvent::Checkpoint { id, label } => SessionEvent::Checkpoint {
            id: id.clone(),
            label: redact_text(label),
        },
        SessionEvent::Info { message } => SessionEvent::Info {
            message: redact_text(message),
        },
        SessionEvent::Verify { command, passed } => SessionEvent::Verify {
            command: redact_text(command),
            passed: *passed,
        },
        SessionEvent::Compaction {
            range_start_seq,
            range_end_seq,
            summary,
        } => SessionEvent::Compaction {
            range_start_seq: *range_start_seq,
            range_end_seq: *range_end_seq,
            summary: redact_text(summary),
        },
    }
}

fn redact_action(action: &Action) -> Action {
    match action {
        Action::Say { text } => Action::Say {
            text: redact_text(text),
        },
        Action::CallTool {
            call_id,
            name,
            args,
        } => Action::CallTool {
            call_id: call_id.clone(),
            name: name.clone(),
            args: redact_text(args),
        },
        Action::Done { summary } => Action::Done {
            summary: redact_text(summary),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derive_skips_control_events() {
        let events = vec![
            SessionEvent::SessionStart {
                session_id: "s".into(),
                format_version: FORMAT_VERSION,
            },
            SessionEvent::UserMessage { text: "hi".into() },
            SessionEvent::ModelResponse {
                text: "hello".into(),
                actions: vec![Action::Say {
                    text: "hello".into(),
                }],
                usage: None,
            },
            SessionEvent::ToolCall {
                call_id: "c1".into(),
                name: "read".into(),
                args: "x".into(),
            },
            SessionEvent::Observation {
                call_id: "c1".into(),
                name: "read".into(),
                output: "ok".into(),
            },
        ];
        let messages = derive_messages(&events);
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0].role, Role::User);
        assert_eq!(messages[1].role, Role::Assistant);
        assert_eq!(messages[2].role, Role::Tool);
    }

    #[test]
    fn redacts_credential_shaped_tokens() {
        let event = SessionEvent::UserMessage {
            text: "use sk-abcdefghijklmnopqrstuvwx now".into(),
        };
        let redacted = redact_event(&event);
        let SessionEvent::UserMessage { text } = redacted else {
            panic!("wrong variant");
        };
        assert!(text.contains("[REDACTED:"));
        assert!(!text.contains("sk-abcdefghijklmnopqrstuvwx"));
    }

    #[test]
    fn error_events_are_control_events() {
        let event = SessionEvent::Error {
            message: "boom".to_string(),
        };
        assert_eq!(event.kind(), "error");
        assert!(derive_messages(std::slice::from_ref(&event)).is_empty());
    }

    #[test]
    fn derive_pairs_tool_calls_and_results() {
        let events = vec![
            SessionEvent::ModelResponse {
                text: "reading".into(),
                actions: vec![Action::CallTool {
                    call_id: "c1".into(),
                    name: "read".into(),
                    args: "{}".into(),
                }],
                usage: None,
            },
            SessionEvent::Observation {
                call_id: "c1".into(),
                name: "read".into(),
                output: "ok".into(),
            },
        ];
        let messages = derive_messages(&events);
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].tool_calls[0].call_id, "c1");
        assert_eq!(messages[1].tool_call_id.as_deref(), Some("c1"));
    }

    #[test]
    fn compaction_resets_history() {
        let events = vec![
            SessionEvent::UserMessage { text: "old".into() },
            SessionEvent::Compaction {
                range_start_seq: 1,
                range_end_seq: 1,
                summary: "earlier".into(),
            },
            SessionEvent::UserMessage { text: "new".into() },
        ];
        let messages = derive_messages(&events);
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, Role::System);
        assert_eq!(messages[1].text, "new");
    }

    #[test]
    fn hash_is_stable() {
        let messages = vec![Message {
            role: Role::User,
            text: "hi".into(),
            tool_call_id: None,
            tool_calls: Vec::new(),
        }];
        assert_eq!(hash_messages(&messages), hash_messages(&messages));
    }

    #[test]
    fn cost_budget_enforces_token_and_dollar_ceilings() {
        let events = vec![
            SessionEvent::ModelResponse {
                text: "step 1".into(),
                actions: Vec::new(),
                usage: Some(TokenUsage {
                    prompt: 100_000,
                    completion: 50_000,
                }),
            },
            SessionEvent::ModelResponse {
                text: "step 2".into(),
                actions: Vec::new(),
                usage: Some(TokenUsage {
                    prompt: 100_000,
                    completion: 50_000,
                }),
            },
        ];
        let total = session_usage(&events);
        assert_eq!(total.prompt, 200_000);
        assert_eq!(total.completion, 100_000);
        assert_eq!(total.total(), 300_000);

        let token_budget = CostBudget {
            max_tokens: Some(250_000),
            max_cost_microdollars: None,
            pricing: ModelPricing::default(),
        };
        assert!(token_budget
            .check(&total)
            .is_some_and(|reason| reason.contains("budget exceeded") && reason.contains("300000")));

        // $3/1M prompt, $15/1M completion => 200k*3 + 100k*15 = 600_000 + 1_500_000 = 2_100_000 microdollars ($2.10)
        let pricing = ModelPricing {
            prompt_microdollars_per_million: 3_000_000,
            completion_microdollars_per_million: 15_000_000,
        };
        assert_eq!(pricing.cost_microdollars(&total), 2_100_000);
        let cost_budget = CostBudget {
            max_tokens: None,
            max_cost_microdollars: Some(500_000), // $0.50 ceiling
            pricing,
        };
        assert!(cost_budget
            .check(&total)
            .is_some_and(|reason| reason.contains("budget exceeded") && reason.contains("0.5000")));
    }
}
