//! The Unix-socket daemon: accept, attach, prompt, detach.

use std::path::PathBuf;
use std::sync::Arc;

use orexis_core::{ContextFile, ToolSpec};
use orexis_ports::{Codec, Provider, SessionRead, ToolExecutor};
use orexis_usecases::{run_turn, ContextConfig, TurnDeps};
use tokio::io::BufReader;
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::{broadcast, Semaphore};

use crate::proto::{read_msg, write_msg, ClientMsg, ServerMsg};
use crate::session::{BroadcastSink, SessionRegistry, SessionRuntime};

pub struct DaemonConfig {
    pub socket: PathBuf,
    pub db: PathBuf,
    pub provider: Arc<dyn Provider>,
    pub tools: Arc<dyn ToolExecutor>,
    pub codec: Arc<dyn Codec>,
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
}

impl Daemon {
    pub fn new(config: DaemonConfig) -> Self {
        let generation = Arc::new(Semaphore::new(config.generation_slots.max(1)));
        let registry = Arc::new(SessionRegistry::new(config.db.clone()));
        Self {
            config: Arc::new(config),
            registry,
            generation,
        }
    }

    pub async fn run(self) -> std::io::Result<()> {
        let listener = UnixListener::bind(&self.config.socket)?;
        loop {
            let (stream, _) = listener.accept().await?;
            let config = self.config.clone();
            let registry = self.registry.clone();
            let generation = self.generation.clone();
            tokio::spawn(async move {
                let _ = handle(stream, config, registry, generation).await;
            });
        }
    }
}

async fn handle(
    stream: UnixStream,
    config: Arc<DaemonConfig>,
    registry: Arc<SessionRegistry>,
    generation: Arc<Semaphore>,
) -> std::io::Result<()> {
    let (read_half, mut write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half);
    let mut line = String::new();
    let mut runtime: Option<Arc<SessionRuntime>> = None;
    let mut live: Option<broadcast::Receiver<ServerMsg>> = None;
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
                        let stored = rt.store.events().await.map_err(io_err)?;
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
                    ClientMsg::Prompt { text } => {
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
                        let config = config.clone();
                        let rt_for_task = rt.clone();
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
                                tools: config.tools.as_ref(),
                                session: rt_for_task.store.as_ref(),
                                reader: rt_for_task.store.as_ref(),
                                sink: sink.as_ref(),
                                session_id: &rt_for_task.id,
                                prompt: &text,
                                max_steps: config.max_steps,
                                context,
                                codec: config.codec.as_ref(),
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
                            if let Ok(mut turn) = rt.turn.lock() {
                                if let Some(handle) = turn.take() {
                                    handle.abort();
                                }
                            };
                        }
                    }
                    ClientMsg::Approve { .. } => {
                        write_msg(&mut write_half, &ServerMsg::Error { message: "approvals land in M3".into() }).await?;
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
