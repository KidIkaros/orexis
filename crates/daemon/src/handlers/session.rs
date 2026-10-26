//! Session handlers: attach, prompt, interrupt, undo, diff.

use std::sync::Arc;

use orexis_adapters_tools::GatedTools;
use orexis_core::SessionEvent;
use orexis_ports::{Checkpointer, ToolExecutor};
use orexis_usecases::{run_turn, ContextConfig, TurnDeps};
use tokio::net::unix::OwnedWriteHalf;
use tokio::sync::{broadcast, Semaphore};

use crate::approvals::{ApprovalHub, HubGate};
use crate::connection::io_err;
use crate::proto::{write_msg, ServerMsg};
use crate::server::DaemonConfig;
use crate::session::{BroadcastDeltaSink, BroadcastSink, SessionRegistry, SessionRuntime};

pub(crate) async fn attach(
    registry: &Arc<SessionRegistry>,
    session: Option<&str>,
    write: &mut OwnedWriteHalf,
    mut last_seq: u64,
) -> std::io::Result<(Arc<SessionRuntime>, broadcast::Receiver<ServerMsg>, u64)> {
    let rt = registry.get_or_create(session).map_err(io_err)?;
    recover_if_unfinished(&rt).await?;
    let receiver = rt.tx.subscribe();
    write_msg(
        write,
        &ServerMsg::Attached {
            session_id: rt.id.clone(),
        },
    )
    .await?;
    let stored = rt.read.events().await.map_err(io_err)?;
    for stored_event in stored {
        write_msg(
            write,
            &ServerMsg::Event {
                seq: stored_event.seq,
                replay: true,
                event: stored_event.event,
            },
        )
        .await?;
        last_seq = last_seq.max(stored_event.seq);
    }
    Ok((rt, receiver, last_seq))
}

pub(crate) struct PromptContext<'a> {
    pub config: &'a Arc<DaemonConfig>,
    pub base_tools: &'a Arc<dyn ToolExecutor>,
    pub hub: &'a Arc<ApprovalHub>,
    pub generation: &'a Arc<Semaphore>,
}

pub(crate) async fn prompt(
    context: PromptContext<'_>,
    rt: Arc<SessionRuntime>,
    text: String,
    skip_verify: bool,
    write: &mut OwnedWriteHalf,
) -> std::io::Result<()> {
    let config = context.config;
    let base_tools = context.base_tools;
    let hub = context.hub;
    let generation = context.generation;
    let permit = match generation.clone().try_acquire_owned() {
        Ok(permit) => permit,
        Err(_) => {
            write_msg(
                write,
                &ServerMsg::Queued {
                    session_id: rt.id.clone(),
                    position: 1,
                },
            )
            .await?;
            generation.clone().acquire_owned().await.map_err(io_err)?
        }
    };
    write_msg(
        write,
        &ServerMsg::Busy {
            session_id: rt.id.clone(),
        },
    )
    .await?;

    let events = rt.read.events().await.map_err(io_err)?;
    let first_turn = !events
        .iter()
        .any(|stored| matches!(stored.event, SessionEvent::ModelRequest { .. }));
    let identity = if first_turn {
        match config.entity.open() {
            Ok(refs) => refs
                .read
                .snapshot()
                .await
                .ok()
                .map(|snapshot| orexis_core::identity_context(&snapshot, 2000))
                .filter(|text| !text.is_empty()),
            Err(_) => None,
        }
    } else {
        None
    };
    let system_prompt = identity
        .map(|text| format!("{}\n\n{}", config.system_prompt, text))
        .unwrap_or_else(|| config.system_prompt.clone());

    let sink = Arc::new(BroadcastSink::new(rt.tx.clone()));
    let delta_sink = Arc::new(BroadcastDeltaSink::new(rt.tx.clone()));
    let gated: Arc<dyn ToolExecutor> = Arc::new(GatedTools::new(
        base_tools.clone(),
        config.policy.clone(),
        Arc::new(HubGate::new(hub.clone(), rt.id.clone())),
    ));
    let config = config.clone();
    let rt_for_task = rt.clone();
    let handle = tokio::spawn(async move {
        let _permit = permit;
        let context = ContextConfig {
            system_prompt: &system_prompt,
            tools: &config.context_tools,
            context_files: &config.context_files,
            context_limit: config.context_limit,
        };
        let _ = run_turn(TurnDeps {
            provider: config.provider.as_ref(),
            tools: gated.as_ref(),
            session: rt_for_task.append.as_ref(),
            reader: rt_for_task.read.as_ref(),
            sink: sink.as_ref(),
            session_id: &rt_for_task.id,
            prompt: &text,
            max_steps: config.max_steps,
            context,
            codec: config.codec.as_ref(),
            checkpointer: config.checkpointer.as_deref(),
            verifier: config.verifier.as_deref(),
            skip_verify,
            delta_sink: delta_sink.as_ref(),
        })
        .await;
        if let Ok(mut turn) = rt_for_task.turn.lock() {
            *turn = None;
        }
    });
    if let Ok(mut turn) = rt.turn.lock() {
        *turn = Some(handle);
    }
    Ok(())
}

pub(crate) async fn self_view(
    config: &Arc<DaemonConfig>,
    write: &mut OwnedWriteHalf,
) -> std::io::Result<()> {
    let refs = config.entity.open().map_err(io_err)?;
    let snapshot = refs.read.snapshot().await.map_err(io_err)?;
    write_msg(
        write,
        &ServerMsg::SelfView {
            text: orexis_usecases::self_view(&snapshot),
        },
    )
    .await
}

pub(crate) async fn approve(
    rt: &Arc<SessionRuntime>,
    hub: &Arc<ApprovalHub>,
    id: u64,
    yes: bool,
    always: bool,
) -> std::io::Result<()> {
    let Some(resolved) = hub.resolve(id, yes, always) else {
        return Ok(());
    };
    if resolved.session != rt.id {
        return Ok(());
    }
    let event = SessionEvent::ApprovalDecision {
        tool: resolved.tool,
        args: resolved.args,
        approved: yes,
        always,
    };
    if let Ok(event_ref) = rt.append.append(event.clone()).await {
        let _ = rt.tx.send(ServerMsg::Event {
            seq: event_ref.seq,
            replay: false,
            event,
        });
    }
    Ok(())
}

pub(crate) async fn audit(
    rt: &Arc<SessionRuntime>,
    write: &mut OwnedWriteHalf,
) -> std::io::Result<()> {
    let events = rt.read.events().await.map_err(io_err)?;
    let mut entries = Vec::new();
    for stored in events {
        let line = match &stored.event {
            SessionEvent::ApprovalDecision {
                tool,
                approved,
                always,
                ..
            } => Some(format!(
                "#{} approval {tool} {} always={always}",
                stored.seq,
                if *approved { "approved" } else { "denied" }
            )),
            SessionEvent::ToolCall { name, .. }
                if matches!(name.as_str(), "shell" | "write_file" | "edit_file") =>
            {
                Some(format!("#{} tool {name}", stored.seq))
            }
            SessionEvent::Checkpoint { id, .. } => Some(format!("#{} checkpoint {id}", stored.seq)),
            SessionEvent::Verify { command, passed } => {
                Some(format!("#{} verify {command} passed={passed}", stored.seq))
            }
            SessionEvent::Error { message } => Some(format!(
                "#{} error {}",
                stored.seq,
                message.chars().take(300).collect::<String>()
            )),
            SessionEvent::Recovered { reason } => {
                Some(format!("#{} recovered {reason}", stored.seq))
            }
            _ => None,
        };
        if let Some(line) = line {
            entries.push(line);
            if entries.len() >= 500 {
                break;
            }
        }
    }
    write_msg(write, &ServerMsg::Audit { entries }).await
}

pub(crate) async fn clear_overrides(rt: &Arc<SessionRuntime>, hub: &Arc<ApprovalHub>) {
    hub.clear_session(&rt.id);
    let event = SessionEvent::Info {
        message: "cleared approval overrides".to_string(),
    };
    if let Ok(event_ref) = rt.append.append(event.clone()).await {
        let _ = rt.tx.send(ServerMsg::Event {
            seq: event_ref.seq,
            replay: false,
            event,
        });
    }
}

pub(crate) async fn interrupt(rt: &Arc<SessionRuntime>) {
    let event = SessionEvent::Interrupted {
        reason: "client interrupt".to_string(),
    };
    if let Ok(event_ref) = rt.append.append(event.clone()).await {
        let _ = rt.tx.send(ServerMsg::Event {
            seq: event_ref.seq,
            replay: false,
            event,
        });
    }
    if let Ok(mut turn) = rt.turn.lock() {
        if let Some(handle) = turn.take() {
            handle.abort();
        }
    }
}

pub(crate) async fn undo(
    rt: &Arc<SessionRuntime>,
    checkpointer: Option<&Arc<dyn Checkpointer>>,
    write: &mut OwnedWriteHalf,
) -> std::io::Result<()> {
    let checkpoint = latest_checkpoint(rt).await?;
    match (checkpoint, checkpointer) {
        (Some(id), Some(checkpointer)) => match checkpointer.restore(&id) {
            Ok(()) => {
                let event = SessionEvent::Info {
                    message: format!("restored checkpoint {id}"),
                };
                if let Ok(event_ref) = rt.append.append(event.clone()).await {
                    let _ = rt.tx.send(ServerMsg::Event {
                        seq: event_ref.seq,
                        replay: false,
                        event,
                    });
                }
            }
            Err(error) => {
                write_msg(
                    write,
                    &ServerMsg::Error {
                        message: error.to_string(),
                    },
                )
                .await?;
            }
        },
        _ => {
            write_msg(
                write,
                &ServerMsg::Error {
                    message: "no checkpoint to undo".into(),
                },
            )
            .await?;
        }
    }
    Ok(())
}

pub(crate) async fn diff(
    rt: &Arc<SessionRuntime>,
    checkpointer: Option<&Arc<dyn Checkpointer>>,
    write: &mut OwnedWriteHalf,
) -> std::io::Result<()> {
    let checkpoint = latest_checkpoint(rt).await?;
    match (checkpoint, checkpointer) {
        (Some(id), Some(checkpointer)) => match checkpointer.diff(&id) {
            Ok(diff) => {
                write_msg(write, &ServerMsg::Diff { diff }).await?;
            }
            Err(error) => {
                write_msg(
                    write,
                    &ServerMsg::Error {
                        message: error.to_string(),
                    },
                )
                .await?;
            }
        },
        _ => {
            write_msg(
                write,
                &ServerMsg::Error {
                    message: "no checkpoint to diff".into(),
                },
            )
            .await?;
        }
    }
    Ok(())
}

pub(crate) async fn latest_checkpoint(rt: &SessionRuntime) -> std::io::Result<Option<String>> {
    let events = rt.read.events().await.map_err(io_err)?;
    Ok(events.iter().rev().find_map(|stored| match &stored.event {
        SessionEvent::Checkpoint { id, .. } => Some(id.clone()),
        _ => None,
    }))
}

async fn recover_if_unfinished(rt: &Arc<SessionRuntime>) -> std::io::Result<()> {
    let active = rt.turn.lock().map(|turn| turn.is_some()).unwrap_or(false);
    if active {
        return Ok(());
    }
    let events = rt.read.events().await.map_err(io_err)?;
    let last = events.iter().rev().find(|stored| {
        !matches!(
            stored.event,
            SessionEvent::Info { .. } | SessionEvent::Compaction { .. }
        )
    });
    let unfinished = last.is_some_and(|stored| {
        !matches!(
            stored.event,
            SessionEvent::Done { .. }
                | SessionEvent::Error { .. }
                | SessionEvent::Interrupted { .. }
                | SessionEvent::Recovered { .. }
        )
    });
    if !unfinished {
        return Ok(());
    }
    let event = SessionEvent::Recovered {
        reason: "daemon restarted with an unfinished turn".to_string(),
    };
    if let Ok(event_ref) = rt.append.append(event.clone()).await {
        let _ = rt.tx.send(ServerMsg::Event {
            seq: event_ref.seq,
            replay: false,
            event,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use orexis_adapters_session_sqlite::SqliteSessionFactory;

    #[tokio::test]
    async fn recovery_marks_an_unfinished_turn_once() {
        let dir = tempfile::tempdir().expect("tempdir");
        let registry =
            SessionRegistry::new(Arc::new(SqliteSessionFactory::new(dir.path().join("s.db"))));
        let rt = registry.get_or_create(None).expect("session");
        rt.append
            .append(SessionEvent::UserMessage {
                text: "start".to_string(),
            })
            .await
            .expect("user");
        rt.append
            .append(SessionEvent::ModelRequest {
                profile_id: "test".to_string(),
                request_hash: "hash".to_string(),
            })
            .await
            .expect("request");

        recover_if_unfinished(&rt).await.expect("recover");
        let events = rt.read.events().await.expect("events");
        assert!(matches!(
            events.last().map(|stored| &stored.event),
            Some(SessionEvent::Recovered { .. })
        ));
        let len = events.len();

        recover_if_unfinished(&rt).await.expect("recover again");
        assert_eq!(rt.read.events().await.expect("events").len(), len);
    }
}
