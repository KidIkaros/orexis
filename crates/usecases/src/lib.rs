//! The turn loop: a use case over core + ports. No adapter, no runtime.

use orexis_core::{
    derive_messages, hash_messages, Action, CodecId, ContextFile, Message, SessionEvent, ToolSpec,
    FORMAT_VERSION,
};
use orexis_ports::{
    Codec, EventSink, PortResult, Provider, SessionAppend, SessionRead, Summarizer, ToolExecutor,
};

pub struct TurnDeps<'a> {
    pub provider: &'a dyn Provider,
    pub tools: &'a dyn ToolExecutor,
    pub session: &'a dyn SessionAppend,
    pub reader: &'a dyn SessionRead,
    pub sink: &'a dyn EventSink,
    pub session_id: &'a str,
    pub prompt: &'a str,
    pub max_steps: usize,
    pub context: ContextConfig<'a>,
    pub codec: &'a dyn Codec,
}

pub struct TurnOutcome {
    pub steps: usize,
    pub done: bool,
}

pub async fn run_turn(deps: TurnDeps<'_>) -> PortResult<TurnOutcome> {
    let TurnDeps {
        provider,
        tools,
        session,
        reader,
        sink,
        session_id,
        prompt,
        max_steps,
        context,
        codec,
    } = deps;

    append(
        session,
        sink,
        SessionEvent::SessionStart {
            session_id: session_id.to_string(),
            format_version: FORMAT_VERSION,
        },
    )
    .await?;
    append(
        session,
        sink,
        SessionEvent::UserMessage {
            text: prompt.to_string(),
        },
    )
    .await?;

    let mut done = false;
    let mut steps = 0;
    let mut reprompts = 0;

    for step in 0..max_steps {
        steps = step + 1;

        let mut events = reader
            .events()
            .await?
            .into_iter()
            .map(|stored| stored.event)
            .collect::<Vec<_>>();
        let tool_fragment = codec.prompt_fragment(context.tools);
        let mut assembled = assemble(&context, &events, &tool_fragment);
        if assembled.needs_compaction {
            if let Some(compaction) = plan_compaction(&events, 8, &ExtractiveSummarizer) {
                append(session, sink, compaction).await?;
                events = reader
                    .events()
                    .await?
                    .into_iter()
                    .map(|stored| stored.event)
                    .collect();
                assembled = assemble(&context, &events, &tool_fragment);
            }
        }
        append(
            session,
            sink,
            SessionEvent::ModelRequest {
                profile_id: "fake".to_string(),
                request_hash: hash_messages(&assembled.messages),
            },
        )
        .await?;

        let response = match provider
            .complete(&assembled.messages, context.tools, step)
            .await
        {
            Ok(response) => response,
            Err(error) => {
                append(
                    session,
                    sink,
                    SessionEvent::Error {
                        message: error.to_string(),
                    },
                )
                .await?;
                return Err(error);
            }
        };

        let decoded = if codec.uses_native_tools() {
            orexis_ports::Decoded {
                actions: response.actions,
                errors: Vec::new(),
            }
        } else {
            codec.decode(&response.raw_text, context.tools)
        };

        if !decoded.errors.is_empty() {
            let note = format!(
                "Malformed action: {}. Emit exactly one valid action.",
                decoded.errors.join("; ")
            );
            append(session, sink, SessionEvent::Reprompt { note }).await?;
            reprompts += 1;
            if reprompts > 2 || step + 1 >= max_steps {
                return Ok(TurnOutcome { steps, done: false });
            }
            continue;
        }
        reprompts = 0;

        let actions = decoded.actions;
        let text = if codec.uses_native_tools() {
            actions
                .iter()
                .filter_map(|action| match action {
                    Action::Say { text } => Some(text.clone()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            response.raw_text.clone()
        };
        append(
            session,
            sink,
            SessionEvent::ModelResponse {
                text,
                actions: actions.clone(),
            },
        )
        .await?;

        for (index, action) in actions.into_iter().enumerate() {
            match action {
                Action::CallTool {
                    call_id,
                    name,
                    args,
                } => {
                    let call_id = if call_id.is_empty() {
                        format!("call-{step}-{index}")
                    } else {
                        call_id
                    };
                    append(
                        session,
                        sink,
                        SessionEvent::ToolCall {
                            call_id: call_id.clone(),
                            name: name.clone(),
                            args: args.clone(),
                        },
                    )
                    .await?;
                    let observation = match tools.execute(&name, &args).await {
                        Ok(observation) => observation,
                        Err(error) => {
                            append(
                                session,
                                sink,
                                SessionEvent::Error {
                                    message: error.to_string(),
                                },
                            )
                            .await?;
                            return Err(error);
                        }
                    };
                    append(
                        session,
                        sink,
                        SessionEvent::Observation {
                            call_id,
                            name: observation.name,
                            output: observation.output,
                        },
                    )
                    .await?;
                }
                Action::Done { summary } => {
                    append(session, sink, SessionEvent::Done { summary }).await?;
                    done = true;
                    break;
                }
                Action::Say { .. } => {}
            }
        }

        if done {
            break;
        }
    }

    Ok(TurnOutcome { steps, done })
}

async fn append(
    session: &dyn SessionAppend,
    sink: &dyn EventSink,
    event: SessionEvent,
) -> PortResult<orexis_core::EventRef> {
    let event_ref = session.append(event.clone()).await?;
    sink.emit(&event);
    Ok(event_ref)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodecCapabilities {
    pub native_tools: bool,
    pub xml_dialect: bool,
    pub grammar: bool,
}

/// ADR-0003: native → known XML dialect → grammar JSON → lenient.
pub fn select_codec(capabilities: CodecCapabilities, override_id: Option<CodecId>) -> CodecId {
    if let Some(id) = override_id {
        return id;
    }
    if capabilities.native_tools {
        CodecId::Native
    } else if capabilities.xml_dialect {
        CodecId::XmlManual
    } else if capabilities.grammar {
        CodecId::Json
    } else {
        CodecId::Lenient
    }
}

pub const COMPACTION_BUFFER_TOKENS: usize = 8192;

pub struct ContextConfig<'a> {
    pub system_prompt: &'a str,
    pub tools: &'a [ToolSpec],
    pub context_files: &'a [ContextFile],
    pub context_limit: usize,
}

pub struct AssembledContext {
    pub system: String,
    pub messages: Vec<Message>,
    pub estimated_tokens: usize,
    pub needs_compaction: bool,
}

pub fn estimate_tokens(text: &str) -> usize {
    text.chars().count() / 4 + 1
}

/// Assemble the model-visible context in a fixed order: system prompt, tool
/// docs, project context, then history. The prefix is stable within a session.
pub fn assemble(
    config: &ContextConfig<'_>,
    events: &[SessionEvent],
    tool_fragment: &str,
) -> AssembledContext {
    let context_fragment = config
        .context_files
        .iter()
        .map(|file| format!("## {}\n{}", file.path, file.content))
        .collect::<Vec<_>>()
        .join("\n\n");
    let system = [
        config.system_prompt,
        tool_fragment,
        context_fragment.as_str(),
    ]
    .into_iter()
    .filter(|part| !part.is_empty())
    .collect::<Vec<_>>()
    .join("\n\n");

    let mut messages = Vec::new();
    if !system.is_empty() {
        messages.push(Message {
            role: orexis_core::Role::System,
            text: system.clone(),
            tool_call_id: None,
            tool_calls: Vec::new(),
        });
    }
    messages.extend(derive_messages(events));
    let estimated_tokens = messages
        .iter()
        .map(|message| estimate_tokens(&message.text))
        .sum::<usize>();
    let limit = config
        .context_limit
        .saturating_sub(COMPACTION_BUFFER_TOKENS);
    let needs_compaction = limit > 0 && estimated_tokens > limit;

    AssembledContext {
        system,
        messages,
        estimated_tokens,
        needs_compaction,
    }
}

/// Plan a compaction that summarizes everything except the last `keep_last`
/// events. The caller appends the returned event.
pub fn plan_compaction(
    events: &[SessionEvent],
    keep_last: usize,
    summarizer: &dyn Summarizer,
) -> Option<SessionEvent> {
    if events.len() <= keep_last {
        return None;
    }
    let split = events.len() - keep_last;
    let summary = summarizer.summarize(&events[..split]);
    Some(SessionEvent::Compaction {
        range_start_seq: 1,
        range_end_seq: split as u64,
        summary,
    })
}

/// Deterministic extractive summarizer for M1; an LLM summarizer replaces it
/// behind the same port later.
pub struct ExtractiveSummarizer;

impl Summarizer for ExtractiveSummarizer {
    fn summarize(&self, events: &[SessionEvent]) -> String {
        let mut parts = Vec::new();
        for event in events {
            match event {
                SessionEvent::UserMessage { text } => {
                    parts.push(format!("user: {}", first_line(text)));
                }
                SessionEvent::ModelResponse { text, .. } if !text.is_empty() => {
                    parts.push(format!("agent: {}", first_line(text)));
                }
                SessionEvent::Done { summary } => {
                    parts.push(format!("done: {}", first_line(summary)));
                }
                SessionEvent::Compaction { summary, .. } => {
                    parts.push(format!("summary: {}", first_line(summary)));
                }
                _ => {}
            }
        }
        parts.join(" | ").chars().take(2000).collect()
    }
}

fn first_line(text: &str) -> &str {
    text.lines().next().unwrap_or("").trim()
}

/// Replay check: every logged `ModelRequest` hash must equal the pure
/// derivation from the events that preceded it.
pub fn verify_replay(events: &[SessionEvent]) -> bool {
    for (index, event) in events.iter().enumerate() {
        if let SessionEvent::ModelRequest { request_hash, .. } = event {
            let messages = derive_messages(&events[..index]);
            if hash_messages(&messages) != *request_hash {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replay_verifies_logged_hashes() {
        let user = SessionEvent::UserMessage { text: "hi".into() };
        let messages = derive_messages(std::slice::from_ref(&user));
        let events = vec![
            SessionEvent::SessionStart {
                session_id: "s".into(),
                format_version: FORMAT_VERSION,
            },
            user,
            SessionEvent::ModelRequest {
                profile_id: "fake".into(),
                request_hash: hash_messages(&messages),
            },
        ];
        assert!(verify_replay(&events));
    }

    #[test]
    fn replay_detects_a_tampered_hash() {
        let events = vec![
            SessionEvent::UserMessage { text: "hi".into() },
            SessionEvent::ModelRequest {
                profile_id: "fake".into(),
                request_hash: "deadbeef".into(),
            },
        ];
        assert!(!verify_replay(&events));
    }

    #[test]
    fn codec_selection_follows_capabilities() {
        let native = CodecCapabilities {
            native_tools: true,
            xml_dialect: true,
            grammar: true,
        };
        assert_eq!(select_codec(native, None), CodecId::Native);

        let xml = CodecCapabilities {
            native_tools: false,
            xml_dialect: true,
            grammar: true,
        };
        assert_eq!(select_codec(xml, None), CodecId::XmlManual);

        let json = CodecCapabilities {
            native_tools: false,
            xml_dialect: false,
            grammar: true,
        };
        assert_eq!(select_codec(json, None), CodecId::Json);

        let lenient = CodecCapabilities {
            native_tools: false,
            xml_dialect: false,
            grammar: false,
        };
        assert_eq!(select_codec(lenient, None), CodecId::Lenient);
        assert_eq!(
            select_codec(native, Some(CodecId::Lenient)),
            CodecId::Lenient
        );
    }

    #[test]
    fn assembly_orders_system_tools_context_and_history() {
        let tools = vec![ToolSpec {
            name: "read".to_string(),
            description: "read a file".to_string(),
            schema: serde_json::json!({"type": "object"}),
        }];
        let context_files = vec![ContextFile {
            path: "AGENTS.md".to_string(),
            content: "rules".to_string(),
        }];
        let events = vec![SessionEvent::UserMessage {
            text: "hi".to_string(),
        }];
        let config = ContextConfig {
            system_prompt: "be helpful",
            tools: &tools,
            context_files: &context_files,
            context_limit: 32768,
        };
        let assembled = assemble(&config, &events, "- read: read a file");
        assert!(assembled.system.starts_with("be helpful"));
        assert!(assembled.system.contains("- read: read a file"));
        assert!(assembled.system.contains("rules"));
        assert_eq!(assembled.messages.len(), 2);
        assert_eq!(assembled.messages[0].role, orexis_core::Role::System);
        assert!(!assembled.needs_compaction);
    }

    #[test]
    fn compaction_keeps_the_tail() {
        let events = vec![
            SessionEvent::UserMessage {
                text: "one".to_string(),
            },
            SessionEvent::ModelResponse {
                text: "two".to_string(),
                actions: Vec::new(),
            },
            SessionEvent::UserMessage {
                text: "three".to_string(),
            },
        ];
        let compaction = plan_compaction(&events, 1, &ExtractiveSummarizer).unwrap();
        let SessionEvent::Compaction {
            range_end_seq,
            summary,
            ..
        } = compaction
        else {
            panic!("wrong variant");
        };
        assert_eq!(range_end_seq, 2);
        assert!(summary.contains("user: one"));
        assert!(summary.contains("agent: two"));
        assert!(!summary.contains("three"));
    }

    #[test]
    fn compaction_is_skipped_when_short() {
        let events = vec![SessionEvent::UserMessage {
            text: "one".to_string(),
        }];
        assert!(plan_compaction(&events, 8, &ExtractiveSummarizer).is_none());
    }
}
