//! Per-connection NDJSON loop.

use std::sync::Arc;

use orexis_ports::ToolExecutor;
use tokio::io::BufReader;
use tokio::net::UnixStream;
use tokio::sync::{broadcast, Semaphore};

use crate::approvals::ApprovalHub;
use crate::handlers;
use crate::proto::{read_msg, write_msg, ClientMsg, ServerMsg};
use crate::server::DaemonConfig;
use crate::session::{SessionRegistry, SessionRuntime};

pub(crate) async fn handle(
    stream: UnixStream,
    config: Arc<DaemonConfig>,
    registry: Arc<SessionRegistry>,
    generation: Arc<Semaphore>,
    tools: Arc<dyn ToolExecutor>,
    hub: Arc<ApprovalHub>,
) -> std::io::Result<()> {
    let (read_half, mut write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half);
    let mut line = String::new();
    let mut runtime: Option<Arc<SessionRuntime>> = None;
    let mut live: Option<broadcast::Receiver<ServerMsg>> = None;
    let mut approvals = hub.subscribe();
    let mut last_seq = 0u64;

    loop {
        tokio::select! {
            incoming = read_msg(&mut reader, &mut line) => {
                let Some(message) = incoming? else { break; };
                match message {
                    ClientMsg::Attach { session } => {
                        let (rt, receiver, seq) = handlers::attach(
                            &registry,
                            session.as_deref(),
                            &mut write_half,
                            last_seq,
                        )
                        .await?;
                        runtime = Some(rt);
                        live = Some(receiver);
                        last_seq = seq;
                    }
                    ClientMsg::Prompt { text, skip_verify } => {
                        let Some(rt) = runtime.clone() else {
                            write_msg(&mut write_half, &ServerMsg::Error { message: "attach first".into() }).await?;
                            continue;
                        };
                        handlers::prompt(
                            &config,
                            &tools,
                            &generation,
                            rt,
                            text,
                            skip_verify,
                            &mut write_half,
                        )
                        .await?;
                    }
                    ClientMsg::Interrupt => {
                        if let Some(rt) = &runtime {
                            handlers::interrupt(rt).await;
                        }
                    }
                    ClientMsg::Approve { id, yes, always } => {
                        hub.resolve(id, yes, always);
                    }
                    ClientMsg::Undo => {
                        let Some(rt) = &runtime else {
                            write_msg(&mut write_half, &ServerMsg::Error { message: "attach first".into() }).await?;
                            continue;
                        };
                        handlers::undo(rt, config.checkpointer.as_ref(), &mut write_half).await?;
                    }
                    ClientMsg::Diff => {
                        let Some(rt) = &runtime else {
                            write_msg(&mut write_half, &ServerMsg::Error { message: "attach first".into() }).await?;
                            continue;
                        };
                        handlers::diff(rt, config.checkpointer.as_ref(), &mut write_half).await?;
                    }
                    ClientMsg::ListSessions => {
                        write_msg(&mut write_half, &ServerMsg::Sessions { sessions: registry.list() }).await?;
                    }
                    ClientMsg::Detach => break,
                }
            }
            message = recv_live(&mut live) => {
                match message {
                    Some(ServerMsg::Event { seq, .. }) if seq <= last_seq => {}
                    Some(message) => {
                        if let ServerMsg::Event { seq, .. } = &message {
                            last_seq = last_seq.max(*seq);
                        }
                        write_msg(&mut write_half, &message).await?;
                    }
                    None => {
                        live = None;
                    }
                }
            }
            message = approvals.recv() => {
                match message {
                    Ok(message) => write_msg(&mut write_half, &message).await?,
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        write_msg(
                            &mut write_half,
                            &ServerMsg::Error {
                                message: format!(
                                    "approval stream lagged: {skipped} requests dropped"
                                ),
                            },
                        )
                        .await?;
                    }
                    Err(broadcast::error::RecvError::Closed) => {}
                }
            }
        }
    }
    Ok(())
}

async fn recv_live(live: &mut Option<broadcast::Receiver<ServerMsg>>) -> Option<ServerMsg> {
    match live {
        Some(receiver) => receiver.recv().await.ok(),
        None => std::future::pending().await,
    }
}

pub(crate) fn io_err(error: impl std::fmt::Display) -> std::io::Error {
    std::io::Error::other(error.to_string())
}
