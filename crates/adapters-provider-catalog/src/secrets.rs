//! Secret resolution.

use std::collections::BTreeMap;

use crate::profile::ProviderProfile;

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
