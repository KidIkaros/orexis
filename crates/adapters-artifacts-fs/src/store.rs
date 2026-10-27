//! The blackboard store and its port.

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;

use async_trait::async_trait;
use orexis_core::Lease;
use orexis_ports::{ArtifactStore, PortError, PortResult};

/// Artifacts are capped — unbounded output is an anti-pattern.
const ARTIFACT_CAP: usize = 8 * 1024 * 1024;

pub struct FsBlackboard {
    root: PathBuf,
    leases: Mutex<BTreeMap<String, Lease>>,
    lease_file: PathBuf,
}

impl FsBlackboard {
    pub fn open(root: impl Into<PathBuf>) -> PortResult<Self> {
        let root = root.into();
        std::fs::create_dir_all(&root)
            .map_err(|error| PortError(format!("blackboard {}: {error}", root.display())))?;
        let lease_file = root.join("leases.json");
        let leases = std::fs::read_to_string(&lease_file)
            .ok()
            .and_then(|text| serde_json::from_str::<BTreeMap<String, Lease>>(&text).ok())
            .unwrap_or_default();
        Ok(Self {
            root,
            leases: Mutex::new(leases),
            lease_file,
        })
    }

    /// A jailed artifact path: no escapes, no absolutes, no empties.
    fn resolve(&self, name: &str) -> PortResult<PathBuf> {
        if name.is_empty() {
            return Err(PortError("empty artifact name".into()));
        }
        let candidate = self.root.join(name);
        let normalized = normalize(&candidate);
        if !normalized.starts_with(&self.root) {
            return Err(PortError(format!("path escapes the blackboard: {name}")));
        }
        Ok(normalized)
    }

    fn persist_leases(&self, leases: &BTreeMap<String, Lease>) -> PortResult<()> {
        let text = serde_json::to_string(leases).map_err(|error| PortError(error.to_string()))?;
        let temp = self.lease_file.with_extension("json.tmp");
        std::fs::write(&temp, text).map_err(|error| PortError(error.to_string()))?;
        std::fs::rename(&temp, &self.lease_file).map_err(|error| PortError(error.to_string()))?;
        Ok(())
    }

    fn now() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
}

fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            Component::Prefix(prefix) => out.push(prefix.as_os_str()),
            Component::RootDir => out.push(Component::RootDir.as_os_str()),
            Component::Normal(part) => out.push(part),
        }
    }
    out
}

#[async_trait]
impl ArtifactStore for FsBlackboard {
    async fn put(&self, name: &str, bytes: &[u8]) -> PortResult<()> {
        let path = self.resolve(name)?;
        if bytes.len() > ARTIFACT_CAP {
            return Err(PortError(format!(
                "artifact {name} is {} bytes, over the {} cap",
                bytes.len(),
                ARTIFACT_CAP
            )));
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| PortError(format!("artifact dir: {error}")))?;
        }
        std::fs::write(&path, bytes).map_err(|error| PortError(format!("artifact {name}: {error}")))
    }

    async fn get(&self, name: &str) -> PortResult<Option<Vec<u8>>> {
        let path = self.resolve(name)?;
        if !path.exists() {
            return Ok(None);
        }
        std::fs::read(&path)
            .map(Some)
            .map_err(|error| PortError(format!("artifact {name}: {error}")))
    }

    async fn list(&self) -> PortResult<Vec<String>> {
        let mut names = Vec::new();
        let mut stack = vec![self.root.clone()];
        while let Some(dir) = stack.pop() {
            let entries = std::fs::read_dir(&dir)
                .map_err(|error| PortError(format!("blackboard: {error}")))?;
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.file_name().and_then(|name| name.to_str()) != Some("leases.json") {
                    if let Ok(relative) = path.strip_prefix(&self.root) {
                        names.push(relative.to_string_lossy().to_string());
                    }
                }
            }
        }
        names.sort();
        Ok(names)
    }

    async fn lease(&self, path: &str, owner: &str, ttl_secs: u64) -> PortResult<Lease> {
        let mut leases = self
            .leases
            .lock()
            .map_err(|_| PortError("leases poisoned".into()))?;
        if let Some(existing) = leases.get(path) {
            if existing.expires_at > Self::now() && existing.owner != owner {
                return Err(PortError(format!("{path} is leased by {}", existing.owner)));
            }
        }
        let lease = Lease {
            path: path.to_string(),
            owner: owner.to_string(),
            expires_at: Self::now() + ttl_secs,
        };
        leases.insert(path.to_string(), lease.clone());
        self.persist_leases(&leases)?;
        Ok(lease)
    }

    async fn steal(&self, path: &str, owner: &str) -> PortResult<Option<Lease>> {
        let mut leases = self
            .leases
            .lock()
            .map_err(|_| PortError("leases poisoned".into()))?;
        match leases.get(path) {
            Some(existing) if existing.expires_at > Self::now() => Ok(None),
            _ => {
                let lease = Lease {
                    path: path.to_string(),
                    owner: owner.to_string(),
                    expires_at: Self::now() + 60,
                };
                leases.insert(path.to_string(), lease.clone());
                self.persist_leases(&leases)?;
                Ok(Some(lease))
            }
        }
    }

    async fn lease_of(&self, path: &str) -> PortResult<Option<Lease>> {
        Ok(self
            .leases
            .lock()
            .map_err(|_| PortError("leases poisoned".into()))?
            .get(path)
            .cloned())
    }
}
