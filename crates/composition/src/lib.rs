//! The composition root: the only place that constructs concrete adapters.

use std::path::PathBuf;
use std::sync::Arc;

use orexis_adapters_codec::{codec_for, NativeCodec};
use orexis_adapters_context_fs::FileContextSource;
use orexis_adapters_entity_sqlite::SqliteEntityFactory;
use orexis_adapters_fake::{
    EchoTools, InMemorySession, RecordingSink, ScriptedProvider, SlowProvider,
};
use orexis_adapters_http::ReqwestTransport;
use orexis_adapters_lsp::StdioLsp;
use orexis_adapters_provider_anthropic::AnthropicProvider;
use orexis_adapters_provider_catalog::{
    Catalog, CatalogError, EnvSecrets, KeyringSecrets, ProviderKind, SecretStore, ToolFormat,
};
use orexis_adapters_provider_openai::OpenAiCompatibleProvider;
use orexis_adapters_session_sqlite::SqliteSessionFactory;
use orexis_adapters_tools::{GitCheckpointStore, LocalTools, ShellVerifier};
use orexis_core::{Action, ContextFile, PermissionPolicy, ToolSpec};
use orexis_ports::{
    ArtifactStore, BoardStore, Checkpointer, Codec, ContextSource, EntityFactory, EvolutionStore,
    ExternalTools, Lsp, MessageStore, PortError, Provider, SessionAppend, SessionFactory,
    SessionRead, ToolExecutor, Verifier, Workspace,
};
use orexis_usecases::{select_codec, CodecCapabilities};

/// The fake wiring used by the vertical slice and tests.
pub struct App {
    pub provider: Arc<dyn Provider>,
    pub tools: Arc<dyn ToolExecutor>,
    pub session: Arc<dyn SessionAppend>,
    pub reader: Arc<dyn SessionRead>,
    pub sink: Arc<RecordingSink>,
}

pub fn build() -> App {
    let provider = Arc::new(ScriptedProvider::new(vec![
        vec![
            Action::Say {
                text: "reading the file".to_string(),
            },
            Action::CallTool {
                call_id: "call-1".to_string(),
                name: "read".to_string(),
                args: "src/lib.rs".to_string(),
            },
        ],
        vec![Action::Done {
            summary: "read and summarized".to_string(),
        }],
    ]));

    let session = Arc::new(InMemorySession::default());
    let session_append: Arc<dyn SessionAppend> = session.clone();
    let session_read: Arc<dyn SessionRead> = session;

    App {
        provider,
        tools: Arc::new(EchoTools),
        session: session_append,
        reader: session_read,
        sink: Arc::new(RecordingSink::default()),
    }
}

pub struct BuildConfig {
    pub profile_id: String,
    pub model_id: String,
    pub db_path: PathBuf,
    pub project_root: PathBuf,
    pub cwd: PathBuf,
    pub providers_dir: Option<PathBuf>,
    pub mcp_dirs: Option<Vec<PathBuf>>,
    pub utcp_dirs: Option<Vec<PathBuf>>,
    pub a2a_dirs: Option<Vec<PathBuf>>,
    /// The hex-encoded evolution signing seed (ADR-0009 env fallback);
    /// `None` means no key is configured.
    pub evolution_key: Option<String>,
    pub verify_command: Option<String>,
    pub system_prompt: String,
    pub context_limit: usize,
}

pub struct Runtime {
    pub provider: Arc<dyn Provider>,
    pub tools: Arc<dyn ToolExecutor>,
    pub workspace: Arc<dyn Workspace>,
    pub codec: Arc<dyn Codec>,
    pub policy: Arc<PermissionPolicy>,
    pub checkpointer: Option<Arc<dyn Checkpointer>>,
    pub verifier: Option<Arc<dyn Verifier>>,
    pub sessions: Arc<dyn SessionFactory>,
    pub entity: Arc<dyn EntityFactory>,
    pub evolution: Option<Arc<dyn EvolutionStore>>,
    pub board: Arc<dyn BoardStore>,
    pub bus: Arc<dyn MessageStore>,
    pub blackboard: Arc<dyn ArtifactStore>,
    pub system_prompt: String,
    pub context_tools: Vec<ToolSpec>,
    pub external: Vec<Arc<dyn ExternalTools>>,
    pub context_files: Vec<ContextFile>,
    pub context_limit: usize,
    pub generation_slots: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error("catalog: {0}")]
    Catalog(#[from] CatalogError),
    #[error("unknown profile/model: {0}/{1}")]
    Unknown(String, String),
    #[error("tools: {0}")]
    Tools(#[from] PortError),
    #[error("unsupported: {0}")]
    Unsupported(String),
}

/// Build the real runtime: catalog -> provider -> codec -> context.
pub fn build_runtime(config: BuildConfig) -> Result<Runtime, BuildError> {
    let mut dirs = vec![providers_dir()?];
    if let Some(user) = dirs::config_dir() {
        dirs.push(user.join("orexis/providers"));
    }
    dirs.push(config.cwd.join(".orexis/providers"));
    if let Some(custom) = &config.providers_dir {
        dirs.push(custom.clone());
    }
    let catalog = Catalog::load(&dirs)?;
    let resolution = catalog
        .resolve_model(&config.profile_id, &config.model_id)
        .ok_or_else(|| BuildError::Unknown(config.profile_id.clone(), config.model_id.clone()))?;
    let profile = resolution.profile.clone();
    let mut warnings = Vec::new();
    if let Some(warning) = resolution.warning.clone() {
        warnings.push(warning);
    }

    // Auth buildability is refused here, naming the kind (ADR-0016: auth
    // work is on demand) — a profile that cannot build must fail at
    // composition, not at the first turn (ticket 014).
    if !auth_kind_implemented(profile.auth.kind) {
        return Err(BuildError::Unsupported(format!(
            "profile {} uses {:?} auth, which is not implemented yet; \
             adapter work is on demand (ADR-0016)",
            profile.id, profile.auth.kind
        )));
    }

    let secret = KeyringSecrets::new("dev.orexis")
        .get(&profile)
        .or_else(|| EnvSecrets.get(&profile));
    let provider: Arc<dyn Provider> = match profile.kind {
        ProviderKind::Anthropic => Arc::new(AnthropicProvider::new(
            profile.clone(),
            config.model_id.clone(),
            secret,
            ReqwestTransport::new(),
        )),
        ProviderKind::OpenAiCompatible | ProviderKind::LlamaCpp => {
            Arc::new(OpenAiCompatibleProvider::new(
                profile.clone(),
                config.model_id.clone(),
                secret,
                ReqwestTransport::new(),
            ))
        }
        ProviderKind::Gemini => {
            return Err(BuildError::Unsupported(
                "the native Gemini wire is on demand; use an openai_compatible profile \
                 (generativelanguage.googleapis.com/v1beta/openai) until a trigger fires"
                    .to_string(),
            ))
        }
        ProviderKind::Bedrock => {
            return Err(BuildError::Unsupported(
                "the bedrock_converse wire is on demand; no compatible endpoint \
                 exists for it (ADR-0016 trigger 1)"
                    .to_string(),
            ))
        }
        ProviderKind::CodexResponses => {
            return Err(BuildError::Unsupported(
                "the codex_responses wire is on demand; no compatible endpoint \
                 exists for it (ADR-0016 trigger 1)"
                    .to_string(),
            ))
        }
    };

    let codec_id = select_codec(
        CodecCapabilities {
            native_tools: profile.wire.tool_format == ToolFormat::Native,
            xml_dialect: profile.wire.tool_format == ToolFormat::Xml,
            grammar: profile.capabilities.json_mode,
        },
        None,
    );
    let codec: Arc<dyn Codec> = Arc::from(codec_for(codec_id));

    let context_files =
        FileContextSource::new(config.project_root.clone(), config.cwd.clone()).load();
    let verifier: Option<Arc<dyn Verifier>> = match &config.verify_command {
        Some(command) => Some(Arc::new(ShellVerifier::new(
            config.project_root.clone(),
            command.clone(),
        )?)),
        None => None,
    };

    let lsp: Arc<dyn Lsp> = Arc::new(StdioLsp::new(config.project_root.clone(), "rust-analyzer"));
    let tools = Arc::new(LocalTools::with_lsp(config.project_root.clone(), lsp)?);
    let workspace: Arc<dyn Workspace> = Arc::new(LocalTools::new(config.project_root.clone())?);
    let entity: Arc<dyn EntityFactory> = Arc::new(SqliteEntityFactory::new(
        config.db_path.with_file_name("entity.db"),
    ));
    let context_tools = LocalTools::specs();
    let mut external = Vec::new();
    if let Some(mcp) = mcp_source(&config, &mut warnings) {
        external.push(mcp);
    }
    if let Some(utcp) = utcp_source(&config, tools.clone(), &mut warnings) {
        external.push(utcp);
    }
    if let Some(a2a) = a2a_source(&config, &mut warnings) {
        external.push(a2a);
    }
    let evolution = evolution_source(&config, &mut warnings);
    let board: Arc<dyn BoardStore> = Arc::new(orexis_adapters_board_sqlite::SqliteBoard::open(
        &config.db_path.with_file_name("board.db"),
    )?);
    let bus: Arc<dyn MessageStore> = Arc::new(orexis_adapters_message_sqlite::SqliteBus::open(
        &config.db_path.with_file_name("bus.db"),
    )?);
    let blackboard: Arc<dyn ArtifactStore> =
        Arc::new(orexis_adapters_artifacts_fs::FsBlackboard::open(
            config.project_root.join(".orexis/blackboard"),
        )?);

    Ok(Runtime {
        provider,
        tools,
        workspace,
        entity,
        evolution,
        board,
        bus,
        blackboard,
        codec,
        policy: Arc::new(PermissionPolicy::standard()),
        checkpointer: Some(Arc::new(GitCheckpointStore::new(
            config.project_root.clone(),
        )?)),
        verifier,
        sessions: Arc::new(SqliteSessionFactory::new(config.db_path)),
        system_prompt: config.system_prompt,
        context_tools,
        external,
        context_files,
        context_limit: config.context_limit,
        generation_slots: profile.limits.max_concurrency.max(1) as usize,
        warnings,
    })
}

/// MCP servers come from explicit dirs, or the user dir plus the project's
/// `.orexis/mcp` (project wins by server name). Problems become warnings;
/// no configured servers means no MCP layer.
fn mcp_source(
    config: &BuildConfig,
    warnings: &mut Vec<String>,
) -> Option<Arc<dyn orexis_ports::ExternalTools>> {
    let dirs = config.mcp_dirs.clone().unwrap_or_else(|| {
        let mut dirs = Vec::new();
        if let Ok(home) = std::env::var("HOME") {
            dirs.push(PathBuf::from(home).join(".config/orexis/mcp"));
        }
        dirs.push(config.project_root.join(".orexis/mcp"));
        dirs
    });
    let (servers, problems) = orexis_adapters_protocol_mcp::load_server_configs(&dirs);
    warnings.extend(problems);
    if servers.is_empty() {
        None
    } else {
        let tools: Arc<orexis_adapters_protocol_mcp::McpTools> =
            Arc::new(orexis_adapters_protocol_mcp::McpTools::new(
                servers,
                Arc::new(orexis_adapters_http::ReqwestTransport::new()),
            ));
        Some(tools)
    }
}

/// UTCP manuals come from explicit dirs, or the user dir plus the
/// project's `.orexis/utcp` (project wins by provider name). The
/// transports are the shared HTTP adapter, the same sandboxed
/// `LocalTools` that serves built-in tools (`CommandRunner`), and the
/// MCP stdio client behind `PeerFactory`.
fn utcp_source(
    config: &BuildConfig,
    commands: Arc<LocalTools>,
    warnings: &mut Vec<String>,
) -> Option<Arc<dyn orexis_ports::ExternalTools>> {
    let dirs = config.utcp_dirs.clone().unwrap_or_else(|| {
        let mut dirs = Vec::new();
        if let Ok(home) = std::env::var("HOME") {
            dirs.push(PathBuf::from(home).join(".config/orexis/utcp"));
        }
        dirs.push(config.project_root.join(".orexis/utcp"));
        dirs
    });
    let (providers, problems) = orexis_adapters_protocol_utcp::load_manuals(&dirs);
    warnings.extend(problems);
    if providers.is_empty() {
        None
    } else {
        let tools: Arc<orexis_adapters_protocol_utcp::UtcpTools> =
            Arc::new(orexis_adapters_protocol_utcp::UtcpTools::new(
                providers,
                Arc::new(orexis_adapters_http::ReqwestTransport::new()),
                commands,
                Arc::new(orexis_adapters_protocol_mcp::StdioPeerFactory {
                    timeout: std::time::Duration::from_secs(10),
                }),
            ));
        Some(tools)
    }
}

/// A2A peers come from explicit dirs, or the user dir plus the project's
/// `.orexis/a2a` (project wins by agent name). Problems become warnings;
/// no configured peers means no A2A layer.
fn a2a_source(
    config: &BuildConfig,
    warnings: &mut Vec<String>,
) -> Option<Arc<dyn orexis_ports::ExternalTools>> {
    let dirs = config.a2a_dirs.clone().unwrap_or_else(|| {
        let mut dirs = Vec::new();
        if let Ok(home) = std::env::var("HOME") {
            dirs.push(PathBuf::from(home).join(".config/orexis/a2a"));
        }
        dirs.push(config.project_root.join(".orexis/a2a"));
        dirs
    });
    let (peers, problems) = orexis_adapters_protocol_a2a::load_peer_configs(&dirs);
    warnings.extend(problems);
    if peers.is_empty() {
        None
    } else {
        let tools: Arc<orexis_adapters_protocol_a2a::A2aTools> =
            Arc::new(orexis_adapters_protocol_a2a::A2aTools::new(
                peers,
                Arc::new(orexis_adapters_http::ReqwestTransport::new()),
            ));
        Some(tools)
    }
}

/// The evolution corridor, when one has been set up (ADR-0015: evolution
/// is off by default, so a fresh install has no corridor and nothing is
/// wired). The signing key comes from the configured seed; a corridor
/// without a key is a visible warning, not a silently hidden overlay.
fn evolution_source(
    config: &BuildConfig,
    warnings: &mut Vec<String>,
) -> Option<Arc<dyn EvolutionStore>> {
    let corridor = config.project_root.join(".orexis/evolution");
    if !corridor.exists() {
        return None;
    }
    let Some(key) = config
        .evolution_key
        .as_deref()
        .filter(|key| !key.is_empty())
    else {
        warnings.push(format!(
            "evolution corridor {} exists but no signing key is configured \
             (OREXIS_EVOLUTION_KEY); /evolve will show no ledger",
            corridor.display()
        ));
        return None;
    };
    let signer = match orexis_adapters_evolution_git::Ed25519Signer::from_hex(key) {
        Ok(signer) => Arc::new(signer),
        Err(error) => {
            warnings.push(format!("evolution signing key: {error}"));
            return None;
        }
    };
    let store: Arc<dyn EvolutionStore> =
        match orexis_adapters_evolution_git::GitCorridor::open(corridor, signer) {
            Ok(store) => Arc::new(store),
            Err(error) => {
                warnings.push(format!("evolution corridor: {error}"));
                return None;
            }
        };
    Some(store)
}

/// Deterministic wiring for tests and `OREXIS_FAKE=1`.
pub fn build_fake_runtime(db_path: PathBuf) -> Runtime {
    let provider: Arc<dyn Provider> = if std::env::var("OREXIS_FAKE_SLOW").is_ok() {
        Arc::new(SlowProvider::new(std::time::Duration::from_secs(30)))
    } else {
        Arc::new(ScriptedProvider::new(vec![vec![Action::Done {
            summary: "fake provider".to_string(),
        }]]))
    };
    Runtime {
        provider,
        tools: Arc::new(EchoTools),
        workspace: Arc::new(NoWorkspace),
        codec: Arc::new(NativeCodec),
        policy: Arc::new(PermissionPolicy::standard()),
        checkpointer: None,
        verifier: None,
        sessions: Arc::new(SqliteSessionFactory::new(db_path.clone())),
        entity: Arc::new(SqliteEntityFactory::new(
            db_path.with_file_name("entity.db"),
        )),
        evolution: None,
        board: Arc::new(orexis_adapters_fake::FakeBoard::new(256)),
        bus: Arc::new(orexis_adapters_fake::FakeBus::new()),
        blackboard: Arc::new(orexis_adapters_fake::FakeBlackboard::new()),
        system_prompt: String::new(),
        context_tools: Vec::new(),
        external: Vec::new(),
        context_files: Vec::new(),
        context_limit: 32768,
        generation_slots: 1,
        warnings: Vec::new(),
    }
}

fn providers_dir() -> Result<PathBuf, BuildError> {
    if let Ok(path) = std::env::var("OREXIS_PROVIDERS") {
        let path = PathBuf::from(path);
        if path.exists() {
            return Ok(path);
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(bin) = exe.parent() {
            for candidate in [bin.join("../share/orexis/providers"), bin.join("providers")] {
                if candidate.exists() {
                    return Ok(candidate);
                }
            }
        }
    }
    let dev =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../adapters-provider-catalog/providers");
    if dev.exists() {
        return Ok(dev);
    }
    Err(BuildError::Unsupported(
        "providers directory not found; set OREXIS_PROVIDERS".to_string(),
    ))
}

struct NoWorkspace;

impl Workspace for NoWorkspace {
    fn list(&self, _path: &str) -> Result<Vec<orexis_ports::FileEntry>, PortError> {
        Ok(Vec::new())
    }

    fn read(&self, path: &str, _max_bytes: usize) -> Result<orexis_ports::FilePreview, PortError> {
        Err(PortError(format!("no workspace file: {path}")))
    }

    fn changed(&self) -> Result<Vec<String>, PortError> {
        Ok(Vec::new())
    }
}

/// The provider selection `orexis init` writes; the environment overrides
/// it (ticket 014).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    pub profile_id: String,
    pub model_id: String,
}

/// Read `.orexis/config.yaml` under the project root, if one is written.
pub fn read_selection(project_root: &std::path::Path) -> Option<Selection> {
    let text = std::fs::read_to_string(project_root.join(".orexis/config.yaml")).ok()?;
    let value: serde_yaml::Value = serde_yaml::from_str(&text).ok()?;
    Some(Selection {
        profile_id: value.get("profile")?.as_str()?.to_string(),
        model_id: value.get("model")?.as_str()?.to_string(),
    })
}

/// Write the selection for `serve` to pick up.
pub fn write_selection(
    project_root: &std::path::Path,
    selection: &Selection,
) -> Result<(), std::io::Error> {
    let dir = project_root.join(".orexis");
    std::fs::create_dir_all(&dir)?;
    std::fs::write(
        dir.join("config.yaml"),
        format!(
            "profile: {}\nmodel: {}\n",
            selection.profile_id, selection.model_id
        ),
    )
}

/// Which auth kinds the adapters can build today (ADR-0016: auth work is
/// on demand). Composition is the honest home — it is the only crate that
/// knows what it constructs; the catalog stays pure data.
pub fn auth_kind_implemented(kind: orexis_adapters_provider_catalog::AuthKind) -> bool {
    use orexis_adapters_provider_catalog::AuthKind;
    matches!(kind, AuthKind::ApiKey | AuthKind::None)
}

/// Load the catalog from the same directories `build_runtime` uses, for
/// the `providers` and `init` surfaces.
pub fn load_catalog() -> Result<orexis_adapters_provider_catalog::Catalog, BuildError> {
    let providers_dir = providers_dir()?;
    let mut dirs = vec![providers_dir];
    if let Some(user) = dirs::config_dir() {
        dirs.push(user.join("orexis/providers"));
    }
    if let Ok(cwd) = std::env::current_dir() {
        dirs.push(cwd.join(".orexis/providers"));
    }
    Ok(orexis_adapters_provider_catalog::Catalog::load(&dirs)?)
}
