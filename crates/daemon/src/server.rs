//! The Unix-socket daemon: accept, attach, prompt, detach.

use std::path::PathBuf;
use std::sync::Arc;

use orexis_adapters_tools::GatedTools;
use orexis_core::{ContextFile, PermissionPolicy, SessionEvent, ToolSpec};
use orexis_ports::{Checkpointer, Codec, Provider, SessionFactory, ToolExecutor, Verifier};
use orexis_usecases::{run_turn, ContextConfig, TurnDeps};
use tokio::io::BufReader;
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::{broadcast, Semaphore};

use crate::approvals::{ApprovalHub, HubGate};
use crate::proto::{read_msg, write_msg, ClientMsg, ServerMsg};
use crate::session::{BroadcastDeltaSink, BroadcastSink, SessionRegistry, SessionRuntime};

pub struct DaemonConfig {
    pub socket: PathBuf,
    pub sessions: Arc<dyn SessionFactory>,
    pub provider: Arc<dyn Provider>,
    pub tools: Arc<dyn ToolExecutor>,
    pub codec: Arc<dyn Codec>,
    pub policy: Arc<PermissionPolicy>,
    pub checkpointer: Option<Arc<dyn Checkpointer>>,
    pub verifier: Option<Arc<dyn Verifier>>,
    pub system_prompt: String,
    pub context_tools: Vec<ToolSpec>,
    pub context_files: Vec<ContextFile>,
    pub context_limit: usize,
    pub max_steps: usize,
    pub generation_slots: usize,
}

pub struct Daemon {
    config: Arc<DaemonConfig>,
    registry: Arc<SessionRegistry>,
    generation: Arc<Semaphore>,
    tools: Arc<dyn ToolExecutor>,
    hub: Arc<ApprovalHub>,
}

impl Daemon {
    pub fn new(config: DaemonConfig) -> Self {
        let generation = Arc::new(Semaphore::new(config.generation_slots.max(1)));
        let registry = Arc::new(SessionRegistry::new(config.sessions.clone()));
        let hub = Arc::new(ApprovalHub::new());
        let tools: Arc<dyn ToolExecutor> = Arc::new(GatedTools::new(
            config.tools.clone(),
            config.policy.clone(),
            Arc::new(HubGate::new(hub.clone())),
        ));
        Self {
            config: Arc::new(config),
            registry,
            generation,
            tools,
            hub,
        }
    }

    pub async fn run(self) -> std::io::Result<()> {
        if self.config.socket.exists() {
            match UnixStream::connect(&self.config.socket).await {
                Ok(_) => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::AddrInUse,
                        "daemon already running",
                    ))
                }
                Err(_) => {
                    let _ = std::fs::remove_file(&self.config.socket);
                }
            }
        }
        let listener = UnixListener::bind(&self.config.socket)?;
        loop {
            let (stream, _) = listener.accept().await?;
            let config = self.config.clone();
            let registry = self.registry.clone();
            let generation = self.generation.clone();
            let tools = self.tools.clone();
            let hub = self.hub.clone();
            tokio::spawn(async move {
                let _ = handle(stream, config, registry, generation, tools, hub).await;
            });
        }
    }
}

async fn handle(
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
                        let rt = registry.get_or_create(session.as_deref()).map_err(io_err)?;
                        let receiver = rt.tx.subscribe();
                        write_msg(&mut write_half, &ServerMsg::Attached { session_id: rt.id.clone() }).await?;
                        let stored = rt.read.events().await.map_err(io_err)?;
                        for stored_event in stored {
                            write_msg(&mut write_half, &ServerMsg::Event {
                                seq: stored_event.seq,
                                replay: true,
                                event: stored_event.event,
                            }).await?;
                            last_seq = last_seq.max(stored_event.seq);
                        }
                        live = Some(receiver);
                        runtime = Some(rt);
                    }
                    ClientMsg::Prompt { text, skip_verify } => {
                        let Some(rt) = runtime.clone() else {
                            write_msg(&mut write_half, &ServerMsg::Error { message: "attach first".into() }).await?;
                            continue;
                        };
                        let permit = match generation.clone().try_acquire_owned() {
                            Ok(permit) => permit,
                            Err(_) => {
                                write_msg(&mut write_half, &ServerMsg::Queued { session_id: rt.id.clone(), position: 1 }).await?;
                                generation.clone().acquire_owned().await.map_err(io_err)?
                            }
                        };
                        write_msg(&mut write_half, &ServerMsg::Busy { session_id: rt.id.clone() }).await?;
                        let sink = Arc::new(BroadcastSink::new(rt.tx.clone()));
                        let delta_sink = Arc::new(BroadcastDeltaSink::new(rt.tx.clone()));
                        let config = config.clone();
                        let rt_for_task = rt.clone();
                        let tools = tools.clone();
                        let handle = tokio::spawn(async move {
                            let _permit = permit;
                            let context = ContextConfig {
                                system_prompt: &config.system_prompt,
                                tools: &config.context_tools,
                                context_files: &config.context_files,
                                context_limit: config.context_limit,
                            };
                            let _ = run_turn(TurnDeps {
                                provider: config.provider.as_ref(),
                                tools: tools.as_ref(),
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
                            }).await;
                            if let Ok(mut turn) = rt_for_task.turn.lock() {
                                *turn = None;
                            }
                        });
                        if let Ok(mut turn) = rt.turn.lock() {
                            *turn = Some(handle);
                        };
                    }
                    ClientMsg::Interrupt => {
                        if let Some(rt) = &runtime {
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
                            };
                        }
                    }
                    ClientMsg::Approve { id, yes } => {
                        hub.resolve(id, yes);
                    }
                    ClientMsg::Undo => {
                        let Some(rt) = &runtime else {
                            write_msg(&mut write_half, &ServerMsg::Error { message: "attach first".into() }).await?;
                            continue;
                        };
                        let events = rt.read.events().await.map_err(io_err)?;
                        let checkpoint = events.iter().rev().find_map(|stored| match &stored.event {
                            SessionEvent::Checkpoint { id, .. } => Some(id.clone()),
                            _ => None,
                        });
                        match (checkpoint, &config.checkpointer) {
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
                                    write_msg(&mut write_half, &ServerMsg::Error { message: error.to_string() }).await?;
                                }
                            },
                            _ => {
                                write_msg(&mut write_half, &ServerMsg::Error { message: "no checkpoint to undo".into() }).await?;
                            }
                        }
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

fn io_err(error: impl std::fmt::Display) -> std::io::Error {
    std::io::Error::other(error.to_string())
}
