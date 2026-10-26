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

pub trait KeyringBackend: Send + Sync {
    fn get(&self, service: &str, account: &str) -> Result<Option<String>, String>;
    fn set(&self, service: &str, account: &str, secret: &str) -> Result<(), String>;
    fn delete(&self, service: &str, account: &str) -> Result<(), String>;
}

/// The OS keyring through the freedesktop `secret-tool` CLI.
#[derive(Debug, Default)]
pub struct SecretToolBackend;

impl KeyringBackend for SecretToolBackend {
    fn get(&self, service: &str, account: &str) -> Result<Option<String>, String> {
        let output = std::process::Command::new("secret-tool")
            .args(["lookup", "service", service, "account", account])
            .output()
            .map_err(|error| error.to_string())?;
        if output.status.success() {
            Ok(Some(
                String::from_utf8_lossy(&output.stdout).trim().to_string(),
            ))
        } else {
            Ok(None)
        }
    }

    fn set(&self, service: &str, account: &str, secret: &str) -> Result<(), String> {
        use std::io::Write;
        let mut child = std::process::Command::new("secret-tool")
            .args([
                "store", "--label", "orexis", "service", service, "account", account,
            ])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|error| error.to_string())?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(secret.as_bytes())
                .map_err(|error| error.to_string())?;
        }
        let status = child.wait().map_err(|error| error.to_string())?;
        if status.success() {
            Ok(())
        } else {
            Err("secret-tool store failed".to_string())
        }
    }

    fn delete(&self, service: &str, account: &str) -> Result<(), String> {
        let _ = std::process::Command::new("secret-tool")
            .args(["clear", "service", service, "account", account])
            .status()
            .map_err(|error| error.to_string())?;
        Ok(())
    }
}

pub struct KeyringSecrets<B: KeyringBackend = SecretToolBackend> {
    backend: B,
    service: String,
}

impl KeyringSecrets<SecretToolBackend> {
    pub fn new(service: impl Into<String>) -> Self {
        Self {
            backend: SecretToolBackend,
            service: service.into(),
        }
    }
}

impl<B: KeyringBackend> KeyringSecrets<B> {
    pub fn with_backend(service: impl Into<String>, backend: B) -> Self {
        Self {
            backend,
            service: service.into(),
        }
    }

    pub fn store(&self, account: &str, secret: &str) -> Result<(), String> {
        self.backend.set(&self.service, account, secret)
    }

    pub fn forget(&self, account: &str) -> Result<(), String> {
        self.backend.delete(&self.service, account)
    }
}

impl<B: KeyringBackend> SecretStore for KeyringSecrets<B> {
    fn get(&self, profile: &ProviderProfile) -> Option<String> {
        self.backend
            .get(&self.service, &profile.id)
            .ok()
            .flatten()
            .filter(|secret| !secret.is_empty())
    }
}
