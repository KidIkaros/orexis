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
    Checkpointer, Codec, ContextSource, EntityFactory, ExternalTools, Lsp, PortError, Provider,
    SessionAppend, SessionFactory, SessionRead, ToolExecutor, Verifier, Workspace,
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
    let providers_dir = match &config.providers_dir {
        Some(dir) => dir.clone(),
        None => providers_dir()?,
    };
    let mut dirs = vec![providers_dir];
    if let Some(user) = dirs::config_dir() {
        dirs.push(user.join("orexis/providers"));
    }
    dirs.push(config.cwd.join(".orexis/providers"));
    let catalog = Catalog::load(&dirs)?;
    let resolution = catalog
        .resolve_model(&config.profile_id, &config.model_id)
        .ok_or_else(|| BuildError::Unknown(config.profile_id.clone(), config.model_id.clone()))?;
    let profile = resolution.profile.clone();
    let mut warnings = Vec::new();
    if let Some(warning) = resolution.warning.clone() {
        warnings.push(warning);
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

    Ok(Runtime {
        provider,
        tools,
        workspace,
        entity,
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
