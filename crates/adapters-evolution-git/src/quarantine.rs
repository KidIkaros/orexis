//! Quarantine: candidate proposals staged without an apply path.

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use orexis_core::{CandidateManifest, Mutation};
use orexis_ports::{MutationQueue, PortError, PortResult, Proposal, ProposalContext};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Stored {
    manifest: CandidateManifest,
    mutation: Mutation,
}

pub struct Quarantine {
    root: PathBuf,
}

impl Quarantine {
    pub fn open(root: impl Into<PathBuf>) -> PortResult<Self> {
        let root = root.into();
        fs::create_dir_all(&root)
            .map_err(|error| PortError(format!("quarantine {}: {error}", root.display())))?;
        Ok(Self { root })
    }

    fn list_sync(&self) -> PortResult<Vec<CandidateManifest>> {
        let mut manifests = Vec::new();
        for entry in fs::read_dir(&self.root)
            .map_err(|error| PortError(format!("quarantine {}: {error}", self.root.display())))?
        {
            let dir = entry.map_err(|error| PortError(error.to_string()))?.path();
            if !dir.is_dir() {
                continue;
            }
            let path = dir.join("candidate.json");
            if !path.exists() {
                continue;
            }
            let text = fs::read_to_string(&path)
                .map_err(|error| PortError(format!("candidate {}: {error}", path.display())))?;
            let stored: Stored =
                serde_json::from_str(&text).map_err(|error| PortError(error.to_string()))?;
            manifests.push(stored.manifest);
        }
        manifests.sort_by(|left, right| left.id.cmp(&right.id));
        Ok(manifests)
    }
}

#[async_trait]
impl MutationQueue for Quarantine {
    async fn propose(
        &self,
        mutation: Mutation,
        context: ProposalContext,
        source: &str,
    ) -> PortResult<Proposal> {
        if !context.boundary {
            return Err(PortError("proposals are session-boundary only".into()));
        }
        mutation.validate().map_err(PortError)?;
        let content_hash = mutation.hash();
        let existing = self.list_sync()?;
        if let Some(manifest) = existing
            .iter()
            .find(|manifest| manifest.content_hash == content_hash)
        {
            return Ok(Proposal {
                manifest: manifest.clone(),
                deduplicated: true,
            });
        }
        if existing
            .iter()
            .any(|manifest| manifest.session_id == context.session_id)
        {
            return Err(PortError("cadence: one proposal per session".into()));
        }
        if existing.iter().any(|manifest| manifest.day == context.day) {
            return Err(PortError("cadence: one proposal per day".into()));
        }
        let manifest = CandidateManifest {
            id: ulid::Ulid::new().to_string(),
            kind: mutation.kind,
            target: mutation.target.clone(),
            base_hash: mutation.base_hash.clone(),
            content_hash,
            size: mutation.size(),
            session_id: context.session_id,
            day: context.day,
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            source: source.to_string(),
        };
        let dir = self.root.join(&manifest.id);
        fs::create_dir_all(&dir)
            .map_err(|error| PortError(format!("candidate {}: {error}", dir.display())))?;
        let stored = Stored {
            manifest: manifest.clone(),
            mutation,
        };
        fs::write(
            dir.join("candidate.json"),
            serde_json::to_vec_pretty(&stored).map_err(|error| PortError(error.to_string()))?,
        )
        .map_err(|error| PortError(error.to_string()))?;
        Ok(Proposal {
            manifest,
            deduplicated: false,
        })
    }

    async fn list(&self) -> PortResult<Vec<CandidateManifest>> {
        self.list_sync()
    }

    async fn get(&self, id: &str) -> PortResult<Option<CandidateManifest>> {
        let path = self.root.join(id).join("candidate.json");
        if !path.exists() {
            return Ok(None);
        }
        let text = fs::read_to_string(&path)
            .map_err(|error| PortError(format!("candidate {}: {error}", path.display())))?;
        let stored: Stored =
            serde_json::from_str(&text).map_err(|error| PortError(error.to_string()))?;
        Ok(Some(stored.manifest))
    }
}
