//! Convert the Hermes provider snapshot (data only) into profiles.

use std::path::Path;

use serde::Deserialize;
use serde_yaml::{Mapping, Value};

use crate::profile::*;
use crate::CatalogError;

#[derive(Debug, Deserialize)]
struct Snapshot {
    source: String,
    #[serde(default)]
    providers: Vec<RawProvider>,
}

#[derive(Debug, Deserialize)]
struct RawProvider {
    name: String,
    display_name: Option<String>,
    base_url: String,
    api_mode: String,
    auth_type: String,
    #[serde(default)]
    env_vars: Vec<String>,
    #[serde(default)]
    aliases: Vec<String>,
    #[serde(default)]
    supports_vision: bool,
    #[serde(default)]
    hidden: bool,
}

pub fn load(path: &Path) -> Result<Vec<ProviderProfile>, CatalogError> {
    let text = std::fs::read_to_string(path)?;
    let snapshot: Snapshot = serde_json::from_str(&text)?;
    let mut profiles = Vec::new();

    for raw in snapshot.providers {
        if raw.hidden || raw.base_url.is_empty() || raw.base_url.contains('{') {
            continue;
        }
        let kind = match raw.api_mode.as_str() {
            "chat_completions" => ProviderKind::OpenAiCompatible,
            "anthropic_messages" => ProviderKind::Anthropic,
            _ => continue,
        };
        let auth_kind = match raw.auth_type.as_str() {
            "api_key" => AuthKind::ApiKey,
            "none" => AuthKind::None,
            _ => continue,
        };

        let env = raw
            .env_vars
            .into_iter()
            .filter(|name| !name.ends_with("_BASE_URL") && name != "BASE_URL")
            .collect::<Vec<_>>();
        let (header, prefix) = if kind == ProviderKind::Anthropic {
            ("x-api-key".to_string(), String::new())
        } else {
            ("authorization".to_string(), "Bearer ".to_string())
        };
        let auth = Auth {
            kind: auth_kind,
            env,
            header,
            prefix,
            keyring: None,
        };
        let wire = Wire {
            dialect: if kind == ProviderKind::Anthropic {
                Dialect::Anthropic
            } else {
                Dialect::OpenAi
            },
            ..Wire::default()
        };
        let capabilities = Capabilities {
            tools: true,
            vision: raw.supports_vision,
            ..Capabilities::default()
        };

        let mut extra = Mapping::new();
        extra.insert(
            Value::String("provenance".to_string()),
            Value::String(snapshot.source.clone()),
        );
        extra.insert(Value::String("verified".to_string()), Value::Bool(false));
        extra.insert(
            Value::String("api_mode".to_string()),
            Value::String(raw.api_mode),
        );
        extra.insert(
            Value::String("auth_type".to_string()),
            Value::String(raw.auth_type),
        );
        if !raw.aliases.is_empty() {
            extra.insert(
                Value::String("aliases".to_string()),
                Value::Sequence(
                    raw.aliases
                        .into_iter()
                        .map(Value::String)
                        .collect::<Vec<_>>(),
                ),
            );
        }

        profiles.push(ProviderProfile {
            id: raw.name.clone(),
            display_name: raw
                .display_name
                .filter(|name| !name.trim().is_empty())
                .unwrap_or_else(|| raw.name.clone()),
            kind,
            base_url: raw.base_url,
            auth,
            wire,
            cache: Cache::default(),
            thinking: Thinking::default(),
            capabilities,
            models: Vec::new(),
            limits: Limits::default(),
            privacy: Privacy::default(),
            fallbacks: Vec::new(),
            extra: Value::Mapping(extra),
        });
    }

    Ok(profiles)
}
