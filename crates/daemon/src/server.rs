//! The Unix-socket daemon: construction and accept loop.

use std::path::PathBuf;
use std::sync::Arc;

use orexis_core::{ContextFile, PermissionPolicy, ToolSpec};
use orexis_ports::{
    ArtifactStore, BoardStore, Checkpointer, Codec, EntityFactory, EvolutionStore, ExternalTools,
    MessageStore, Provider, SessionFactory, ToolExecutor, Verifier, Workspace,
};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::Semaphore;

use crate::approvals::{ApprovalHub, MemoryOverrides};
use crate::connection;
use crate::session::SessionRegistry;

#[derive(Clone)]
pub struct DaemonConfig {
    pub socket: PathBuf,
    pub profile_id: String,
    pub model_id: String,
    pub sessions: Arc<dyn SessionFactory>,
    pub entity: Arc<dyn EntityFactory>,
    pub identity_path: Option<PathBuf>,
    /// The evolution corridor through its port; `None` is "no evolution"
    /// (off by default per ADR-0015).
    pub evolution: Option<Arc<dyn EvolutionStore>>,
    /// The team domain's stores (M8, ADR-0020).
    pub board: Arc<dyn BoardStore>,
    pub bus: Arc<dyn MessageStore>,
    pub blackboard: Arc<dyn ArtifactStore>,
    /// Team mode is opt-in (invariant 19); off is today's single-agent
    /// behavior.
    pub team: bool,
    pub external: Vec<Arc<dyn ExternalTools>>,
    pub provider: Arc<dyn Provider>,
    pub tools: Arc<dyn ToolExecutor>,
    pub workspace: Arc<dyn Workspace>,
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
    base_tools: Arc<dyn ToolExecutor>,
    hub: Arc<ApprovalHub>,
}

impl Daemon {
    pub fn new(mut config: DaemonConfig) -> Self {
        let generation = Arc::new(Semaphore::new(config.generation_slots.max(1)));
        let registry = Arc::new(SessionRegistry::new(config.sessions.clone()));
        // Team mode gains the board tools: the model's write surface for
        // the board (M8-11), inside the same permission gate. The prompt
        // says so — tools a model cannot discover are tools it won't use.
        let base_tools: Arc<dyn ToolExecutor> = if config.team {
            config.system_prompt.push_str(
                "\n\nThis is a team session. You may decompose work onto the shared board \
                 with board_add (title, strategy: plan, build, review, or mine; optional \
                 depends_on) and read it with board_list. Workers claim board tasks and \
                 run them under your oversight.",
            );
            let board = config.board.clone();
            config
                .context_tools
                .extend(orexis_adapters_tools::TeamTools::specs());
            Arc::new(orexis_adapters_tools::TeamTools::new(
                board,
                config.tools.clone(),
                orexis_usecases::StrategyId::ALL
                    .iter()
                    .map(|id| id.name().to_string())
                    .collect(),
            ))
        } else {
            config.tools.clone()
        };
        let hub = Arc::new(ApprovalHub::new(Arc::new(MemoryOverrides::default())));
        Self {
            config: Arc::new(config),
            registry,
            generation,
            base_tools,
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
        // Team mode is opt-in (invariant 19): the loop exists only when
        // it is on; off is today's single-agent behavior.
        if self.config.team {
            let loop_self = Arc::new(crate::team::TeamLoop::new(
                self.config.clone(),
                self.registry.clone(),
                self.base_tools.clone(),
                self.hub.clone(),
                self.generation.clone(),
            ));
            loop_self.spawn();
        }
        loop {
            let (stream, _) = listener.accept().await?;
            let config = self.config.clone();
            let registry = self.registry.clone();
            let generation = self.generation.clone();
            let base_tools = self.base_tools.clone();
            let hub = self.hub.clone();
            tokio::spawn(async move {
                let _ =
                    connection::handle(stream, config, registry, generation, base_tools, hub).await;
            });
        }
    }
}
