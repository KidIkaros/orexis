//! The ACP server: editors and hosts drive Orexis over stdio (ADR-0017 §5).
//!
//! One ACP connection owns its sessions; `session/prompt` runs the same
//! turn machinery as the socket daemon (gate, router, MCP, identity) and
//! maps the broadcast stream to `session/update` notifications. The drive
//! runs as a task so the read loop keeps consuming — `session/cancel`
//! arrives mid-prompt.

use std::collections::HashMap;
use std::sync::Arc;

use orexis_adapters_jsonrpc::Correlations;
use orexis_adapters_protocol_acp::{
    encode_line, initialize_response, method_not_found, outcome_from, parse_incoming,
    permission_request, prompt_response, session_new_response, stop_reason_from, update_from,
    AcpError, AcpIncoming, Message,
};
use orexis_ports::ToolExecutor;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::Semaphore;

use crate::approvals::{ApprovalHub, MemoryOverrides};
use crate::handlers::{ensure_external, interrupt};
use crate::proto::ServerMsg;
use crate::server::DaemonConfig;
use crate::session::SessionRegistry;
use crate::team::spawn_turn;

/// A client answer we are waiting for: which approval it resolves.
#[derive(Debug, Clone)]
struct PendingPermission {
    approval_id: u64,
}

pub struct AcpServer {
    config: Arc<DaemonConfig>,
    registry: Arc<SessionRegistry>,
    hub: Arc<ApprovalHub>,
    generation: Arc<Semaphore>,
    base_tools: Arc<dyn ToolExecutor>,
    sessions: std::sync::Mutex<HashMap<String, Arc<crate::session::SessionRuntime>>>,
    drives: tokio::sync::Mutex<Vec<tokio::task::JoinHandle<()>>>,
    /// Outgoing client requests awaiting answers (permission round-trips).
    pending: Arc<std::sync::Mutex<Correlations<PendingPermission>>>,
    next_client_id: Arc<std::sync::atomic::AtomicU64>,
}

impl AcpServer {
    pub fn new(config: DaemonConfig) -> Self {
        let config = Arc::new(config);
        let registry = Arc::new(SessionRegistry::new(config.sessions.clone()));
        let hub = Arc::new(ApprovalHub::new(Arc::new(MemoryOverrides::default())));
        let generation = Arc::new(Semaphore::new(config.generation_slots.max(1)));
        let base_tools = config.tools.clone();
        Self {
            config,
            registry,
            hub,
            generation,
            base_tools,
            sessions: std::sync::Mutex::new(HashMap::new()),
            drives: tokio::sync::Mutex::new(Vec::new()),
            pending: Arc::new(std::sync::Mutex::new(Correlations::new())),
            next_client_id: Arc::new(std::sync::atomic::AtomicU64::new(1)),
        }
    }

    pub async fn run<W: AsyncWrite + Unpin + Send + 'static>(
        &self,
        reader: &mut (impl AsyncBufRead + Unpin),
        out: W,
    ) -> std::io::Result<W> {
        let out = Arc::new(tokio::sync::Mutex::new(out));
        let forwarder = self.spawn_permission_forwarder(out.clone());
        let mut line = String::new();
        loop {
            line.clear();
            if reader.read_line(&mut line).await? == 0 {
                // Wait for in-flight drives so their output is flushed and
                // the writer can be handed back (tests read the transcript).
                forwarder.abort();
                let _ = forwarder.await;
                let drives = self.drives.lock().await.drain(..).collect::<Vec<_>>();
                for drive in drives {
                    let _ = drive.await;
                }
                return Ok(Arc::try_unwrap(out)
                    .map_err(|_| std::io::Error::other("writer still shared"))?
                    .into_inner());
            }
            self.handle_line(&line, out.clone()).await?;
        }
    }

    /// Approval requests become `session/request_permission` requests to
    /// the client; the client's answer resolves the approval hub.
    fn spawn_permission_forwarder<W: AsyncWrite + Unpin + Send + 'static>(
        &self,
        out: Arc<tokio::sync::Mutex<W>>,
    ) -> tokio::task::JoinHandle<()> {
        let hub = self.hub.clone();
        let pending = self.pending.clone();
        let next_id = self.next_client_id.clone();
        let mut approvals = hub.subscribe();
        tokio::spawn(async move {
            loop {
                match approvals.recv().await {
                    Ok(ServerMsg::ApprovalRequest {
                        id,
                        session_id,
                        tool,
                        ..
                    }) => {
                        let request_id = orexis_adapters_jsonrpc::RequestId::Number(
                            next_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst) as i64,
                        );
                        let registered = pending
                            .lock()
                            .expect("pending")
                            .insert(request_id.clone(), PendingPermission { approval_id: id });
                        if !registered {
                            continue;
                        }
                        let request = permission_request(request_id, &session_id, id, &tool);
                        if write_message(&out, &request).await.is_err() {
                            return;
                        }
                    }
                    Ok(_) => {}
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(_) => return,
                }
            }
        })
    }

    pub async fn handle_line<W: AsyncWrite + Unpin + Send + 'static>(
        &self,
        line: &str,
        out: Arc<tokio::sync::Mutex<W>>,
    ) -> std::io::Result<()> {
        let incoming = match parse_incoming(line) {
            Ok(incoming) => incoming,
            Err(AcpError::Parse(error)) => {
                return error_response(&out, None, format!("malformed ACP wire: {error}")).await;
            }
            Err(AcpError::Shape(message)) => {
                return error_response(&out, None, message).await;
            }
        };
        match incoming {
            AcpIncoming::Initialize { id } => write_message(&out, &initialize_response(id)).await,
            AcpIncoming::SessionNew { id } => match self.registry.get_or_create(None) {
                Ok(rt) => {
                    ensure_external(&rt, &self.config)
                        .await
                        .map_err(std::io::Error::other)?;
                    self.sessions
                        .lock()
                        .expect("sessions")
                        .insert(rt.id.clone(), rt.clone());
                    write_message(&out, &session_new_response(id, &rt.id)).await
                }
                Err(error) => error_response(&out, Some(id), error.to_string()).await,
            },
            AcpIncoming::SessionPrompt {
                id,
                session_id,
                text,
            } => {
                let rt = self
                    .sessions
                    .lock()
                    .expect("sessions")
                    .get(&session_id)
                    .cloned();
                match rt {
                    Some(rt) => {
                        // Subscribe before spawning: a cancel processed by
                        // the read loop must not outrun the drive's
                        // subscription.
                        let receiver = rt.tx.subscribe();
                        let pieces = DrivePieces {
                            config: self.config.clone(),
                            base_tools: self.base_tools.clone(),
                            hub: self.hub.clone(),
                            generation: self.generation.clone(),
                        };
                        let drive = tokio::spawn(async move {
                            let stop = match drive_prompt(&pieces, &rt, text, receiver, &out).await
                            {
                                Ok(stop) => stop,
                                Err(error) => {
                                    let _ = error_response(&out, Some(id), error.to_string()).await;
                                    return;
                                }
                            };
                            let _ = write_message(&out, &prompt_response(id, stop)).await;
                        });
                        self.drives.lock().await.push(drive);
                        Ok(())
                    }
                    None => {
                        error_response(&out, Some(id), format!("unknown session {session_id}"))
                            .await
                    }
                }
            }
            AcpIncoming::SessionCancel { session_id } => {
                let rt = self
                    .sessions
                    .lock()
                    .expect("sessions")
                    .get(&session_id)
                    .cloned();
                if let Some(rt) = rt {
                    interrupt(&rt).await;
                }
                Ok(())
            }
            AcpIncoming::ClientResponse { id, result, error } => {
                let Some(id) = id else {
                    return Ok(());
                };
                // On a real wire the answer follows the request; the
                // recorded fixtures compress time, so the answer may be
                // read before the forwarder registers the request. Wait
                // briefly for the registration before treating it as stray.
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
                let approval_id = loop {
                    let answered = self
                        .pending
                        .lock()
                        .expect("pending")
                        .take(&id)
                        .map(|pending| pending.approval_id);
                    if answered.is_some() || std::time::Instant::now() >= deadline {
                        break answered;
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                };
                let Some(approval_id) = approval_id else {
                    return Ok(());
                };
                let (yes, always) = outcome_from(result.as_ref(), error.as_ref());
                let _ = self.hub.resolve(approval_id, yes, always);
                Ok(())
            }
            AcpIncoming::Unhandled { id, method } => {
                write_message(&out, &method_not_found(id, &method)).await
            }
        }
    }
}

async fn write_message<W: AsyncWrite + Unpin>(
    out: &Arc<tokio::sync::Mutex<W>>,
    message: &Message,
) -> std::io::Result<()> {
    let mut out = out.lock().await;
    out.write_all(encode_line(message).as_bytes()).await?;
    out.flush().await
}

async fn error_response<W: AsyncWrite + Unpin>(
    out: &Arc<tokio::sync::Mutex<W>>,
    id: Option<orexis_adapters_jsonrpc::RequestId>,
    message: String,
) -> std::io::Result<()> {
    let response = Message::Response(orexis_adapters_jsonrpc::Response::error(
        id,
        orexis_adapters_jsonrpc::ErrorObject {
            code: -32603,
            message,
            data: None,
        },
    ));
    write_message(out, &response).await
}

/// The pieces a prompt drive needs from the server; cloned per drive.
struct DrivePieces {
    config: Arc<DaemonConfig>,
    base_tools: Arc<dyn ToolExecutor>,
    hub: Arc<ApprovalHub>,
    generation: Arc<Semaphore>,
}

/// Run one prompt: spawn the turn, map the broadcast stream to ACP
/// updates, and return the stop reason.
async fn drive_prompt<W: AsyncWrite + Unpin>(
    pieces: &DrivePieces,
    rt: &Arc<crate::session::SessionRuntime>,
    text: String,
    mut receiver: tokio::sync::broadcast::Receiver<crate::proto::ServerMsg>,
    out: &Arc<tokio::sync::Mutex<W>>,
) -> std::io::Result<&'static str> {
    let permit = pieces
        .generation
        .clone()
        .acquire_owned()
        .await
        .map_err(std::io::Error::other)?;
    let mut handle = spawn_turn(
        rt,
        &pieces.config,
        &pieces.base_tools,
        &pieces.hub,
        text,
        false,
        permit,
    )
    .await
    .map_err(std::io::Error::other)?;
    let mut stop = "end_turn";
    loop {
        tokio::select! {
            message = receiver.recv() => {
                match message {
                    Ok(message) => {
                        if let Some(update) = update_from(&message, &rt.id) {
                            write_message(out, &update).await?;
                        }
                        if let Some(reason) = stop_reason_from(&message) {
                            stop = reason;
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(_) => break,
                }
            }
            _ = &mut handle => {
                // The turn ended without a terminal event; drain what is
                // queued so the stop reason is still the real one.
                while let Ok(message) = receiver.try_recv() {
                    if let Some(update) = update_from(&message, &rt.id) {
                        write_message(out, &update).await?;
                    }
                    if let Some(reason) = stop_reason_from(&message) {
                        stop = reason;
                    }
                }
                break;
            }
        }
    }
    // The select may have already polled the handle to completion in a
    // losing branch; awaiting a consumed JoinHandle panics.
    if !handle.is_finished() {
        handle.abort();
        let _ = handle.await;
    }
    Ok(stop)
}
