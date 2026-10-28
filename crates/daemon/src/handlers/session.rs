//! Session handlers: attach, prompt, interrupt, undo, diff.

use std::path::Path;
use std::sync::Arc;

use orexis_core::{EntitySnapshot, SessionEvent};
use tokio::net::unix::OwnedWriteHalf;
use tokio::sync::{broadcast, Semaphore};

use crate::approvals::ApprovalHub;
use crate::connection::io_err;
use crate::proto::{write_msg, ServerMsg};
use crate::server::DaemonConfig;
use crate::session::{ExternalState, SessionRegistry, SessionRuntime};
use crate::team::spawn_turn;
use orexis_ports::{Checkpointer, ToolExecutor};

pub(crate) async fn attach(
    registry: &Arc<SessionRegistry>,
    config: &Arc<DaemonConfig>,
    session: Option<&str>,
    write: &mut OwnedWriteHalf,
    mut last_seq: u64,
) -> std::io::Result<(Arc<SessionRuntime>, broadcast::Receiver<ServerMsg>, u64)> {
    let rt = registry.get_or_create(session).map_err(io_err)?;
    ensure_external(&rt, config).await?;
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

/// Connect the configured external tool sources once per session, before
/// the client sees any replay, so a failed source is a visible session
/// event.
pub(crate) async fn ensure_external(
    rt: &Arc<SessionRuntime>,
    config: &Arc<DaemonConfig>,
) -> std::io::Result<()> {
    if config.external.is_empty() {
        return Ok(());
    }
    let mut state = rt.external.lock().await;
    if !matches!(*state, ExternalState::NotTried) {
        return Ok(());
    }
    let mut specs = Vec::new();
    let mut failures = 0;
    for source in &config.external {
        for (name, error) in source.connect_all().await {
            failures += 1;
            append_session_event(
                rt,
                SessionEvent::Error {
                    message: format!("{} source {name}: {error}", source.kind()),
                },
            )
            .await?;
        }
        specs.extend(source.specs().await);
    }
    *state = if specs.is_empty() && failures > 0 {
        ExternalState::Failed
    } else {
        ExternalState::Connected(specs)
    };
    Ok(())
}

async fn append_session_event(rt: &SessionRuntime, event: SessionEvent) -> std::io::Result<()> {
    let event_ref = rt.append.append(event.clone()).await.map_err(io_err)?;
    let _ = rt.tx.send(ServerMsg::Event {
        seq: event_ref.seq,
        replay: false,
        event,
    });
    Ok(())
}

/// Assemble the turn context (identity, router, gate, MCP specs) and spawn
/// the turn task. Shared by the socket prompt handler and the ACP server.
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

    let handle = spawn_turn(&rt, config, base_tools, hub, text, skip_verify, permit)
        .await
        .map_err(io_err)?;
    if let Ok(mut turn) = rt.turn.lock() {
        *turn = Some(handle);
    }
    Ok(())
}

pub(crate) async fn board(
    config: &Arc<DaemonConfig>,
    write: &mut OwnedWriteHalf,
) -> std::io::Result<()> {
    let tasks = config.board.list().await.unwrap_or_default();
    write_msg(write, &ServerMsg::Board { tasks }).await
}

/// The fleet is the board's live workers: tasks a worker has claimed
/// (running) carry their owner; finished workers drop out.
pub(crate) async fn fleet(
    config: &Arc<DaemonConfig>,
    write: &mut OwnedWriteHalf,
) -> std::io::Result<()> {
    let workers = config
        .board
        .list()
        .await
        .unwrap_or_default()
        .into_iter()
        .filter(|task| task.status == orexis_core::TaskStatus::Running)
        .collect();
    write_msg(write, &ServerMsg::Fleet { workers }).await
}

pub(crate) async fn evolution(
    config: &Arc<DaemonConfig>,
    write: &mut OwnedWriteHalf,
) -> std::io::Result<()> {
    let mut entries = Vec::new();
    if let Some(store) = &config.evolution {
        // The path and format live in the corridor adapter; the overlay
        // keeps the most recent entries so a growing ledger never hides
        // the newest decisions.
        match store.export_jsonl() {
            Ok(text) => {
                let lines: Vec<&str> = text
                    .lines()
                    .filter(|line| !line.trim().is_empty())
                    .collect();
                let start = lines.len().saturating_sub(500);
                entries = lines[start..].iter().map(|line| line.to_string()).collect();
            }
            Err(error) => eprintln!("warning: evolution ledger: {error}"),
        }
    }
    write_msg(write, &ServerMsg::Evolution { entries }).await
}

pub(crate) async fn self_view(
    config: &Arc<DaemonConfig>,
    write: &mut OwnedWriteHalf,
) -> std::io::Result<()> {
    let refs = config.entity.open().map_err(io_err)?;
    let snapshot = refs.read.snapshot().await.map_err(io_err)?;
    let mut text = orexis_usecases::self_view(&snapshot);
    if let Some(path) = &config.identity_path {
        if let Err(error) = write_identity_file(path, &snapshot) {
            text.push_str(&format!("identity file not written: {error}\n"));
        }
    }
    write_msg(write, &ServerMsg::SelfView { text }).await
}

fn write_identity_file(path: &Path, snapshot: &EntitySnapshot) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, orexis_core::render_identity(snapshot))
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
