//! Executing decoded actions: tool calls and completion.

use orexis_core::{Action, SessionEvent};
use orexis_ports::{EventSink, PortResult, SessionAppend, ToolExecutor};

use crate::session::append;

/// Execute actions in order. Returns `true` when a `Done` action was seen.
pub(crate) async fn execute_actions(
    session: &dyn SessionAppend,
    sink: &dyn EventSink,
    tools: &dyn ToolExecutor,
    actions: Vec<Action>,
    step: usize,
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
                append(session, sink, SessionEvent::Done { summary }).await?;
                return Ok(true);
            }
            Action::Say { .. } => {}
        }
    }
    Ok(false)
}
