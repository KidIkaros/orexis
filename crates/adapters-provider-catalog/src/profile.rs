//! Provider profile types.

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
