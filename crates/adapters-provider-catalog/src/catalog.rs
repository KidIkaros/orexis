//! The vendored catalog and model resolution.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::profile::*;

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
