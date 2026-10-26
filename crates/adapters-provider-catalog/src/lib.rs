//! Provider profiles, the vendored catalog, and secret resolution (ADR-0002).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProviderKind {
    #[serde(rename = "openai_compatible")]
    OpenAiCompatible,
    #[serde(rename = "anthropic")]
    Anthropic,
    #[serde(rename = "gemini")]
    Gemini,
    #[serde(rename = "llamacpp")]
    LlamaCpp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthKind {
    #[default]
    ApiKey,
    OAuth,
    Adc,
    SigV4,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Auth {
    #[serde(default)]
    pub kind: AuthKind,
    #[serde(default)]
    pub env: Vec<String>,
    #[serde(default = "default_header")]
    pub header: String,
    #[serde(default = "default_prefix")]
    pub prefix: String,
    #[serde(default)]
    pub keyring: Option<String>,
}

fn default_header() -> String {
    "authorization".to_string()
}

fn default_prefix() -> String {
    "Bearer ".to_string()
}

impl Default for Auth {
    fn default() -> Self {
        Self {
            kind: AuthKind::ApiKey,
            env: Vec::new(),
            header: default_header(),
            prefix: default_prefix(),
            keyring: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Dialect {
    #[default]
    OpenAi,
    Anthropic,
    Gemini,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolFormat {
    #[default]
    Native,
    Xml,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StreamFormat {
    #[default]
    Sse,
    Jsonl,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Wire {
    #[serde(default)]
    pub dialect: Dialect,
    #[serde(default)]
    pub tool_format: ToolFormat,
    #[serde(default)]
    pub stream_format: StreamFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CacheDialect {
    #[default]
    None,
    Anthropic,
    OpenAi,
    Gemini,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Cache {
    #[serde(default)]
    pub dialect: CacheDialect,
    #[serde(default)]
    pub breakpoints: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Thinking {
    #[serde(default)]
    pub supported: bool,
    #[serde(default)]
    pub param: Option<String>,
    #[serde(default)]
    pub levels: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Capabilities {
    #[serde(default)]
    pub tools: bool,
    #[serde(default)]
    pub parallel_tools: bool,
    #[serde(default)]
    pub vision: bool,
    #[serde(default)]
    pub json_mode: bool,
    #[serde(default)]
    pub reasoning: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Pricing {
    #[serde(default)]
    pub input: f64,
    #[serde(default)]
    pub output: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    #[serde(default)]
    pub context: u32,
    #[serde(default)]
    pub max_output: u32,
    #[serde(default)]
    pub pricing: Pricing,
    #[serde(default)]
    pub capabilities: Capabilities,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Limits {
    #[serde(default = "default_concurrency")]
    pub max_concurrency: u32,
    #[serde(default = "default_retries")]
    pub retries: u32,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

fn default_concurrency() -> u32 {
    4
}
fn default_retries() -> u32 {
    2
}
fn default_timeout() -> u64 {
    120
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_concurrency: default_concurrency(),
            retries: default_retries(),
            timeout_secs: default_timeout(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Privacy {
    #[serde(default)]
    pub data_policy: Option<String>,
    #[serde(default)]
    pub retention: Option<String>,
    #[serde(default)]
    pub region: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderProfile {
    pub id: String,
    pub display_name: String,
    pub kind: ProviderKind,
    pub base_url: String,
    #[serde(default)]
    pub auth: Auth,
    #[serde(default)]
    pub wire: Wire,
    #[serde(default)]
    pub cache: Cache,
    #[serde(default)]
    pub thinking: Thinking,
    #[serde(default)]
    pub capabilities: Capabilities,
    #[serde(default)]
    pub models: Vec<ModelInfo>,
    #[serde(default)]
    pub limits: Limits,
    #[serde(default)]
    pub privacy: Privacy,
    #[serde(default)]
    pub fallbacks: Vec<String>,
    #[serde(default)]
    pub extra: serde_yaml::Value,
}

#[derive(Debug, thiserror::Error)]
pub enum CatalogError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("yaml: {0}")]
    Yaml(#[from] serde_yaml::Error),
}

#[derive(Debug, Default)]
pub struct Catalog {
    profiles: BTreeMap<String, ProviderProfile>,
    sources: BTreeMap<String, PathBuf>,
}

impl Catalog {
    /// Load `*.yaml`/`*.yml` from each directory in order; later directories win.
    pub fn load(dirs: &[PathBuf]) -> Result<Self, CatalogError> {
        let mut catalog = Self::default();
        for dir in dirs {
            if !dir.exists() {
                continue;
            }
            let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)?
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .collect();
            entries.sort();
            for path in entries {
                let is_yaml = path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| ext == "yaml" || ext == "yml");
                if !is_yaml {
                    continue;
                }
                let text = std::fs::read_to_string(&path)?;
                let profile: ProviderProfile = serde_yaml::from_str(&text)?;
                catalog.sources.insert(profile.id.clone(), path);
                catalog.profiles.insert(profile.id.clone(), profile);
            }
        }
        Ok(catalog)
    }

    pub fn get(&self, id: &str) -> Option<&ProviderProfile> {
        self.profiles.get(id)
    }

    pub fn list(&self) -> impl Iterator<Item = &ProviderProfile> {
        self.profiles.values()
    }

    pub fn source_of(&self, id: &str) -> Option<&Path> {
        self.sources.get(id).map(PathBuf::as_path)
    }
}

pub struct ModelResolution<'a> {
    pub profile: &'a ProviderProfile,
    pub model: ModelInfo,
    pub known: bool,
    pub warning: Option<String>,
}

impl Catalog {
    /// Resolve a model, synthesizing conservative defaults when unknown.
    pub fn resolve_model(&self, profile_id: &str, model_id: &str) -> Option<ModelResolution<'_>> {
        let profile = self.get(profile_id)?;
        if let Some(model) = profile.models.iter().find(|model| model.id == model_id) {
            return Some(ModelResolution {
                profile,
                model: model.clone(),
                known: true,
                warning: None,
            });
        }
        Some(ModelResolution {
            profile,
            model: ModelInfo {
                id: model_id.to_string(),
                context: 32768,
                max_output: 4096,
                pricing: Pricing::default(),
                capabilities: Capabilities {
                    tools: profile.capabilities.tools,
                    ..Capabilities::default()
                },
            },
            known: false,
            warning: Some(format!(
                "unknown model '{model_id}' for provider '{profile_id}'; using conservative defaults"
            )),
        })
    }
}

pub trait SecretStore: Send + Sync {
    fn get(&self, profile: &ProviderProfile) -> Option<String>;
}

#[derive(Debug, Default)]
pub struct EnvSecrets;

impl SecretStore for EnvSecrets {
    fn get(&self, profile: &ProviderProfile) -> Option<String> {
        profile
            .auth
            .env
            .iter()
            .find_map(|name| std::env::var(name).ok())
    }
}

#[derive(Debug, Default)]
pub struct MemorySecrets {
    values: BTreeMap<String, String>,
}

impl MemorySecrets {
    pub fn new(values: BTreeMap<String, String>) -> Self {
        Self { values }
    }
}

impl SecretStore for MemorySecrets {
    fn get(&self, profile: &ProviderProfile) -> Option<String> {
        self.values.get(&profile.id).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OPENAI: &str = r#"
id: openai
display_name: OpenAI
kind: openai_compatible
base_url: https://api.openai.com/v1
auth:
  env: [OPENAI_API_KEY]
models:
  - id: gpt-5
    context: 400000
"#;

    #[test]
    fn parses_a_profile() {
        let profile: ProviderProfile = serde_yaml::from_str(OPENAI).unwrap();
        assert_eq!(profile.id, "openai");
        assert_eq!(profile.auth.header, "authorization");
        assert_eq!(profile.limits.max_concurrency, 4);
        assert_eq!(profile.models[0].context, 400000);
    }

    #[test]
    fn later_directories_win() {
        let shipped = tempfile::tempdir().unwrap();
        let user = tempfile::tempdir().unwrap();
        std::fs::write(shipped.path().join("openai.yaml"), OPENAI).unwrap();
        let override_yaml = OPENAI.replace("gpt-5", "gpt-5-mini");
        std::fs::write(user.path().join("openai.yaml"), override_yaml).unwrap();

        let catalog =
            Catalog::load(&[shipped.path().to_path_buf(), user.path().to_path_buf()]).unwrap();
        let profile = catalog.get("openai").unwrap();
        assert_eq!(profile.models[0].id, "gpt-5-mini");
        assert!(catalog
            .source_of("openai")
            .unwrap()
            .starts_with(user.path()));
    }

    #[test]
    fn env_secret_resolves() {
        std::env::set_var("OREXIS_TEST_KEY_123", "secret-value");
        let profile: ProviderProfile =
            serde_yaml::from_str(&OPENAI.replace("OPENAI_API_KEY", "OREXIS_TEST_KEY_123")).unwrap();
        assert_eq!(EnvSecrets.get(&profile), Some("secret-value".to_string()));
        std::env::remove_var("OREXIS_TEST_KEY_123");
    }

    #[test]
    fn memory_secret_resolves() {
        let profile: ProviderProfile = serde_yaml::from_str(OPENAI).unwrap();
        let mut values = BTreeMap::new();
        values.insert("openai".to_string(), "mem".to_string());
        let store = MemorySecrets::new(values);
        assert_eq!(store.get(&profile), Some("mem".to_string()));
    }

    #[test]
    fn unknown_model_gets_conservative_defaults() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("openai.yaml"), OPENAI).unwrap();
        let catalog = Catalog::load(&[dir.path().to_path_buf()]).unwrap();
        let resolution = catalog.resolve_model("openai", "not-a-model").unwrap();
        assert!(!resolution.known);
        assert_eq!(resolution.model.context, 32768);
        assert!(resolution.warning.is_some());
    }
}
