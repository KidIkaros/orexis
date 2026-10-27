//! The turn loop: a use case over core + ports. No adapter, no runtime.

use orexis_core::{hash_messages, Action, SessionEvent, Thinking, FORMAT_VERSION};
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
            },
        )
        .await?;

        if execute_actions(session, sink, tools, actions, step, verifier, skip_verify).await? {
            done = true;
            break;
        }
    }

    Ok(TurnOutcome { steps, done })
}
