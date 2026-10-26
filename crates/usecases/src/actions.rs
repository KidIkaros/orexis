//! Executing decoded actions: tool calls and completion.

use orexis_core::{Action, SessionEvent};
use orexis_ports::{EventSink, PortResult, SessionAppend, ToolExecutor, Verifier};

use crate::session::append;

/// Execute actions in order. Returns `true` when a `Done` action was seen.
pub(crate) async fn execute_actions(
    session: &dyn SessionAppend,
    sink: &dyn EventSink,
    tools: &dyn ToolExecutor,
    actions: Vec<Action>,
    step: usize,
    verifier: Option<&dyn Verifier>,
    skip_verify: bool,
) -> PortResult<bool> {
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
                if let Some(verifier) = verifier {
                    if !skip_verify {
                        let result = verifier.verify().await?;
                        append(
                            session,
                            sink,
                            SessionEvent::Verify {
                                command: result.command.clone(),
                                passed: result.passed,
                            },
                        )
                        .await?;
                        if !result.passed {
                            append(
                                session,
                                sink,
                                SessionEvent::Error {
                                    message: format!("verify failed:\n{}", result.output),
                                },
                            )
                            .await?;
                            append(
                                session,
                                sink,
                                SessionEvent::Reprompt {
                                    note: "Verification failed. Fix the errors, then finish again. Ask the user if skipping is appropriate.".to_string(),
                                },
                            )
                            .await?;
                            return Ok(false);
                        }
                    }
                }
                append(session, sink, SessionEvent::Done { summary }).await?;
                return Ok(true);
            }
            Action::Say { .. } => {}
        }
    }
    Ok(false)
}
