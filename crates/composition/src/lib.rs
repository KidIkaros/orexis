//! The composition root: the only place that constructs concrete adapters.

use std::path::PathBuf;
use std::sync::Arc;

use orexis_adapters_codec::{codec_for, NativeCodec};
use orexis_adapters_context_fs::FileContextSource;
use orexis_adapters_fake::{EchoTools, InMemorySession, RecordingSink, ScriptedProvider};
use orexis_adapters_http::ReqwestTransport;
use orexis_adapters_lsp::StdioLsp;
use orexis_adapters_provider_anthropic::AnthropicProvider;
use orexis_adapters_provider_catalog::{
    Catalog, CatalogError, EnvSecrets, ProviderKind, SecretStore, ToolFormat,
};
use orexis_adapters_provider_openai::OpenAiCompatibleProvider;
use orexis_adapters_session_sqlite::SqliteSessionFactory;
use orexis_adapters_tools::{GitCheckpointStore, LocalTools, ShellVerifier};
use orexis_core::{Action, ContextFile, PermissionPolicy, ToolSpec};
use orexis_ports::{
    Checkpointer, Codec, ContextSource, Lsp, PortError, Provider, SessionAppend, SessionFactory,
    SessionRead, ToolExecutor, Verifier,
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
    pub verify_command: Option<String>,
    pub system_prompt: String,
    pub context_limit: usize,
}

pub struct Runtime {
    pub provider: Arc<dyn Provider>,
    pub tools: Arc<dyn ToolExecutor>,
    pub codec: Arc<dyn Codec>,
    pub policy: Arc<PermissionPolicy>,
    pub checkpointer: Option<Arc<dyn Checkpointer>>,
    pub verifier: Option<Arc<dyn Verifier>>,
    pub sessions: Arc<dyn SessionFactory>,
    pub system_prompt: String,
    pub context_tools: Vec<ToolSpec>,
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

    let secret = EnvSecrets.get(&profile);
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
                "the native Gemini adapter lands in M1-07".to_string(),
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
    let context_tools = LocalTools::specs();

    Ok(Runtime {
        provider,
        tools,
        codec,
        policy: Arc::new(PermissionPolicy::standard()),
        checkpointer: Some(Arc::new(GitCheckpointStore::new(
            config.project_root.clone(),
        )?)),
        verifier,
        sessions: Arc::new(SqliteSessionFactory::new(config.db_path)),
        system_prompt: config.system_prompt,
        context_tools,
        context_files,
        context_limit: config.context_limit,
        generation_slots: profile.limits.max_concurrency.max(1) as usize,
        warnings,
    })
}

/// Deterministic wiring for tests and `OREXIS_FAKE=1`.
pub fn build_fake_runtime(db_path: PathBuf) -> Runtime {
    let provider = Arc::new(ScriptedProvider::new(vec![vec![Action::Done {
        summary: "fake provider".to_string(),
    }]]));
    Runtime {
        provider,
        tools: Arc::new(EchoTools),
        codec: Arc::new(NativeCodec),
        policy: Arc::new(PermissionPolicy::standard()),
        checkpointer: None,
        verifier: None,
        sessions: Arc::new(SqliteSessionFactory::new(db_path)),
        system_prompt: String::new(),
        context_tools: Vec::new(),
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
