//! The Unix-socket daemon: construction and accept loop.

use std::path::PathBuf;
use std::sync::Arc;

use orexis_adapters_tools::GatedTools;
use orexis_core::{ContextFile, PermissionPolicy, ToolSpec};
use orexis_ports::{Checkpointer, Codec, Provider, SessionFactory, ToolExecutor, Verifier};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::Semaphore;

use crate::approvals::{ApprovalHub, HubGate};
use crate::connection;
use crate::session::SessionRegistry;

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
                let _ = connection::handle(stream, config, registry, generation, tools, hub).await;
            });
        }
    }
}
