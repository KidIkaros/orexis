//! The turn loop: a use case over core + ports. No adapter, no runtime.

use orexis_core::{
    hash_messages, session_usage, Action, CostBudget, SessionEvent, Thinking, FORMAT_VERSION,
};
use orexis_ports::{
    Checkpointer, Codec, DeltaSink, EventSink, PortResult, Provider, SessionAppend, SessionRead,
    ToolExecutor, Verifier,
};

use crate::actions::execute_actions;
use crate::compaction::{plan_compaction, ExtractiveSummarizer};
use crate::context::{assemble, ContextConfig};
use crate::session::append;

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
    pub checkpointer: Option<&'a dyn Checkpointer>,
    pub verifier: Option<&'a dyn Verifier>,
    pub skip_verify: bool,
    pub delta_sink: &'a dyn DeltaSink,
    /// The turn-boundary thinking request; the provider conditions its
    /// wire on it (per-provider conditioning, PLAN §4.4).
    pub thinking: Thinking,
}

pub struct TurnOutcome {
    pub steps: usize,
    pub done: bool,
}

pub async fn run_turn(deps: TurnDeps<'_>) -> PortResult<TurnOutcome> {
    run_turn_with_budget(deps, CostBudget::default(), 8).await
}

pub async fn run_turn_with_budget(
    deps: TurnDeps<'_>,
    cost_budget: CostBudget,
    compaction_keep_tail: usize,
) -> PortResult<TurnOutcome> {
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
        checkpointer,
        verifier,
        skip_verify,
        delta_sink,
        thinking,
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

    if let Some(checkpointer) = checkpointer {
        match checkpointer.create("turn") {
            Ok(id) => {
                append(
                    session,
                    sink,
                    SessionEvent::Checkpoint {
                        id,
                        label: "turn".to_string(),
                    },
                )
                .await?;
            }
            Err(error) => {
                append(
                    session,
                    sink,
                    SessionEvent::Error {
                        message: error.to_string(),
                    },
                )
                .await?;
            }
        }
    }

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
        if let Some(reason) = cost_budget.check(&session_usage(&events)) {
            append(session, sink, SessionEvent::Interrupted { reason }).await?;
            return Ok(TurnOutcome { steps, done: false });
        }
        let tool_fragment = codec.prompt_fragment(context.tools);
        let mut assembled = assemble(&context, &events, &tool_fragment);
        if assembled.needs_compaction {
            let keep_last = compaction_keep_tail.max(1);
            if let Some(compaction) = plan_compaction(&events, keep_last, &ExtractiveSummarizer) {
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
            .stream(
                &assembled.messages,
                context.tools,
                step,
                thinking,
                delta_sink,
            )
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
                usage: response.usage,
            },
        )
        .await?;

        let mut cumulative = session_usage(&events);
        if let Some(step_usage) = &response.usage {
            cumulative.accumulate(step_usage);
        }
        if let Some(reason) = cost_budget.check(&cumulative) {
            append(session, sink, SessionEvent::Interrupted { reason }).await?;
            return Ok(TurnOutcome { steps, done: false });
        }

        if execute_actions(session, sink, tools, actions, step, verifier, skip_verify).await? {
            done = true;
            break;
        }
    }

    Ok(TurnOutcome { steps, done })
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use futures::executor::block_on;
    use orexis_core::{
        CostBudget, EventRef, Message, ModelPricing, Observation, ProviderResponse, StoredEvent,
        TokenUsage, ToolSpec,
    };
    use orexis_ports::{Decoded, NullDeltaSink};
    use std::sync::Mutex;

    #[derive(Default)]
    struct MemSession {
        events: Mutex<Vec<StoredEvent>>,
    }

    #[async_trait]
    impl SessionAppend for MemSession {
        async fn append(&self, event: SessionEvent) -> PortResult<EventRef> {
            let mut events = self.events.lock().unwrap();
            let seq = events.len() as u64 + 1;
            let event_id = format!("ev-{seq}");
            events.push(StoredEvent {
                seq,
                event_id: event_id.clone(),
                generation_id: 1,
                event,
            });
            Ok(EventRef { seq, event_id })
        }
    }

    #[async_trait]
    impl SessionRead for MemSession {
        async fn events(&self) -> PortResult<Vec<StoredEvent>> {
            Ok(self.events.lock().unwrap().clone())
        }
    }

    struct Sink;
    impl EventSink for Sink {
        fn emit(&self, _event: &SessionEvent, _seq: u64) {}
    }

    struct PassCodec;
    impl Codec for PassCodec {
        fn name(&self) -> &'static str {
            "pass"
        }
        fn uses_native_tools(&self) -> bool {
            true
        }
        fn prompt_fragment(&self, _tools: &[ToolSpec]) -> String {
            String::new()
        }
        fn decode(&self, _raw: &str, _tools: &[ToolSpec]) -> Decoded {
            Decoded {
                actions: Vec::new(),
                errors: Vec::new(),
            }
        }
    }

    struct ExpensiveProvider;
    #[async_trait]
    impl Provider for ExpensiveProvider {
        async fn complete(
            &self,
            _messages: &[Message],
            _tools: &[ToolSpec],
            _step: usize,
            _thinking: Thinking,
        ) -> PortResult<ProviderResponse> {
            Ok(ProviderResponse {
                actions: vec![Action::CallTool {
                    call_id: "c1".into(),
                    name: "noop".into(),
                    args: "{}".into(),
                }],
                raw_text: String::new(),
                usage: Some(TokenUsage {
                    prompt: 100_000,
                    completion: 50_000,
                }),
            })
        }
    }

    struct NoopTools;
    #[async_trait]
    impl ToolExecutor for NoopTools {
        async fn execute(&self, name: &str, _args: &str) -> PortResult<Observation> {
            Ok(Observation {
                name: name.to_string(),
                output: "ok".into(),
            })
        }
    }

    #[test]
    fn turn_halts_immediately_when_cost_budget_is_exceeded() {
        let session = MemSession::default();
        let budget = CostBudget {
            max_tokens: None,
            max_cost_microdollars: Some(500_000), // $0.50 ceiling
            pricing: ModelPricing {
                prompt_microdollars_per_million: 3_000_000,
                completion_microdollars_per_million: 15_000_000,
            },
        };
        let outcome = block_on(run_turn_with_budget(
            TurnDeps {
                provider: &ExpensiveProvider,
                tools: &NoopTools,
                session: &session,
                reader: &session,
                sink: &Sink,
                session_id: "s1",
                prompt: "spend money",
                max_steps: 5,
                context: ContextConfig {
                    system_prompt: "sys",
                    tools: &[],
                    context_files: &[],
                    context_limit: 32768,
                },
                codec: &PassCodec,
                checkpointer: None,
                verifier: None,
                skip_verify: false,
                delta_sink: &NullDeltaSink,
                thinking: Thinking::default(),
            },
            budget,
            4,
        ))
        .expect("turn");

        assert!(!outcome.done);
        assert_eq!(outcome.steps, 1);
        let events = block_on(session.events()).unwrap();
        assert!(events.iter().any(|stored| matches!(
            &stored.event,
            SessionEvent::Interrupted { reason } if reason.contains("budget exceeded")
        )));
    }
}
