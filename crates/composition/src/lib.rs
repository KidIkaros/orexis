//! The composition root: the only place that constructs concrete adapters.

use std::path::PathBuf;
use std::sync::Arc;

use orexis_adapters_codec::codec_for;
use orexis_adapters_context_fs::FileContextSource;
use orexis_adapters_fake::{EchoTools, InMemorySession, RecordingSink, ScriptedProvider};
use orexis_adapters_http::ReqwestTransport;
use orexis_adapters_provider_anthropic::AnthropicProvider;
use orexis_adapters_provider_catalog::{
    Catalog, CatalogError, EnvSecrets, ProviderKind, SecretStore, ToolFormat,
};
use orexis_adapters_provider_openai::OpenAiCompatibleProvider;
use orexis_core::{Action, ContextFile, ToolSpec};
use orexis_ports::{Codec, ContextSource, Provider, SessionAppend, SessionRead, ToolExecutor};
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
    pub system_prompt: String,
    pub context_limit: usize,
}

pub struct Runtime {
    pub provider: Arc<dyn Provider>,
    pub tools: Arc<dyn ToolExecutor>,
    pub codec: Arc<dyn Codec>,
    pub system_prompt: String,
    pub context_tools: Vec<ToolSpec>,
    pub context_files: Vec<ContextFile>,
    pub context_limit: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error("catalog: {0}")]
    Catalog(#[from] CatalogError),
    #[error("unknown profile/model: {0}/{1}")]
    Unknown(String, String),
    #[error("unsupported: {0}")]
    Unsupported(String),
}

/// Build the real runtime: catalog → provider → codec → SQLite → context.
pub fn build_runtime(config: BuildConfig) -> Result<Runtime, BuildError> {
    let shipped =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../adapters-provider-catalog/providers");
    let mut dirs = vec![shipped];
    if let Some(user) = dirs::config_dir() {
        dirs.push(user.join("orexis/providers"));
    }
    dirs.push(config.cwd.join(".orexis/providers"));
    let catalog = Catalog::load(&dirs)?;
    let resolution = catalog
        .resolve_model(&config.profile_id, &config.model_id)
        .ok_or_else(|| BuildError::Unknown(config.profile_id.clone(), config.model_id.clone()))?;
    let profile = resolution.profile.clone();
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
            xml_dialect: false,
            grammar: profile.capabilities.json_mode,
        },
        None,
    );
    let codec: Arc<dyn Codec> = Arc::from(codec_for(codec_id));

    let context_files =
        FileContextSource::new(config.project_root.clone(), config.cwd.clone()).load();

    Ok(Runtime {
        provider,
        tools: Arc::new(EchoTools),
        codec,
        system_prompt: config.system_prompt,
        context_tools: Vec::new(),
        context_files,
        context_limit: config.context_limit,
    })
}
