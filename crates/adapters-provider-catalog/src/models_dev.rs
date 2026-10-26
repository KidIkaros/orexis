//! Convert the models.dev snapshot into provider profiles and model metadata.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;
use serde_yaml::{Mapping, Value};

use crate::profile::*;
use crate::CatalogError;

#[derive(Debug, Default, Deserialize)]
struct RawLimit {
    context: Option<u64>,
    output: Option<u64>,
}

#[derive(Debug, Default, Deserialize)]
struct RawCost {
    input: Option<f64>,
    output: Option<f64>,
}

#[derive(Debug, Default, Deserialize)]
struct RawModel {
    id: Option<String>,
    tool_call: Option<bool>,
    structured_output: Option<bool>,
    reasoning: Option<bool>,
    attachment: Option<bool>,
    #[serde(default)]
    limit: RawLimit,
    #[serde(default)]
    cost: RawCost,
}

#[derive(Debug, Default, Deserialize)]
struct RawProvider {
    name: Option<String>,
    api: Option<String>,
    #[serde(default)]
    env: Vec<String>,
    npm: Option<String>,
    doc: Option<String>,
    #[serde(default)]
    models: BTreeMap<String, RawModel>,
}

pub struct ModelsDev {
    pub profiles: Vec<ProviderProfile>,
    pub models: BTreeMap<String, Vec<ModelInfo>>,
}

pub fn load(path: &Path) -> Result<ModelsDev, CatalogError> {
    let text = std::fs::read_to_string(path)?;
    let raw: BTreeMap<String, RawProvider> = serde_json::from_str(&text)?;
    let mut profiles = Vec::new();
    let mut models = BTreeMap::new();

    for (id, provider) in raw {
        let converted: Vec<ModelInfo> = provider
            .models
            .iter()
            .map(|(model_id, model)| convert_model(model_id, model))
            .collect();
        let capabilities = aggregate_capabilities(&converted);
        models.insert(id.clone(), converted.clone());

        let Some(api) = provider
            .api
            .filter(|api| api.starts_with("http") && !api.contains('{'))
        else {
            continue;
        };

        let auth = Auth {
            env: provider.env.clone(),
            ..Auth::default()
        };
        let thinking = Thinking {
            supported: capabilities.reasoning,
            ..Thinking::default()
        };

        let mut extra = Mapping::new();
        extra.insert(
            Value::String("provenance".to_string()),
            Value::String("models.dev@2026-10-26".to_string()),
        );
        extra.insert(Value::String("verified".to_string()), Value::Bool(false));
        if let Some(npm) = provider.npm {
            extra.insert(Value::String("npm".to_string()), Value::String(npm));
        }
        if let Some(doc) = provider.doc {
            extra.insert(Value::String("doc".to_string()), Value::String(doc));
        }

        profiles.push(ProviderProfile {
            id: id.clone(),
            display_name: provider
                .name
                .filter(|name| !name.trim().is_empty())
                .unwrap_or_else(|| id.clone()),
            kind: ProviderKind::OpenAiCompatible,
            base_url: api,
            auth,
            wire: Wire::default(),
            cache: Cache::default(),
            thinking,
            capabilities: capabilities.clone(),
            models: converted,
            limits: Limits::default(),
            privacy: Privacy::default(),
            fallbacks: Vec::new(),
            extra: Value::Mapping(extra),
        });
    }

    Ok(ModelsDev { profiles, models })
}

fn convert_model(id: &str, model: &RawModel) -> ModelInfo {
    ModelInfo {
        id: model.id.clone().unwrap_or_else(|| id.to_string()),
        context: model.limit.context.unwrap_or(32768).min(u32::MAX as u64) as u32,
        max_output: model.limit.output.unwrap_or(4096).min(u32::MAX as u64) as u32,
        pricing: Pricing {
            input: model.cost.input.unwrap_or(0.0),
            output: model.cost.output.unwrap_or(0.0),
        },
        capabilities: Capabilities {
            tools: model.tool_call.unwrap_or(false),
            parallel_tools: false,
            vision: model.attachment.unwrap_or(false),
            json_mode: model.structured_output.unwrap_or(false),
            reasoning: model.reasoning.unwrap_or(false),
        },
    }
}

fn aggregate_capabilities(models: &[ModelInfo]) -> Capabilities {
    Capabilities {
        tools: models.iter().any(|model| model.capabilities.tools),
        parallel_tools: false,
        vision: models.iter().any(|model| model.capabilities.vision),
        json_mode: models.iter().any(|model| model.capabilities.json_mode),
        reasoning: models.iter().any(|model| model.capabilities.reasoning),
    }
}
