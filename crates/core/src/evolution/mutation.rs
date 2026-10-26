//! Data-only mutation proposals and candidate manifests.

use serde::{Deserialize, Serialize};

pub const MAX_MUTATION_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MutationKind {
    PromptFragment,
    GeneParameter,
    SkillContent,
    RecallPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mutation {
    pub kind: MutationKind,
    pub target: String,
    pub base_hash: String,
    pub content: String,
}

impl Mutation {
    pub fn hash(&self) -> String {
        let bytes = serde_json::to_vec(&(self.kind, &self.target, &self.base_hash, &self.content))
            .unwrap_or_default();
        blake3::hash(&bytes).to_hex().to_string()
    }

    pub fn size(&self) -> usize {
        self.content.len()
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.target.trim().is_empty() {
            return Err("mutation target is empty".into());
        }
        if self.base_hash.trim().is_empty() {
            return Err("mutation base hash is empty".into());
        }
        if self.content.trim().is_empty() {
            return Err("mutation content is empty".into());
        }
        if self.content.len() > MAX_MUTATION_BYTES {
            return Err(format!(
                "mutation content exceeds {MAX_MUTATION_BYTES} bytes"
            ));
        }
        let lower = self.content.to_lowercase();
        for marker in [
            "#!",
            "subprocess.",
            "os.system",
            "eval(",
            "exec(",
            "curl | sh",
            "base64 -d",
        ] {
            if lower.contains(marker) {
                return Err(format!("mutation contains executable marker: {marker}"));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Candidate {
    pub manifest: CandidateManifest,
    pub mutation: Mutation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateManifest {
    pub id: String,
    pub kind: MutationKind,
    pub target: String,
    pub base_hash: String,
    pub content_hash: String,
    pub size: usize,
    pub session_id: String,
    pub day: u64,
    pub created_at: u64,
    pub source: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mutation(content: &str) -> Mutation {
        Mutation {
            kind: MutationKind::GeneParameter,
            target: "drives.curiosity".to_string(),
            base_hash: "base".to_string(),
            content: content.to_string(),
        }
    }

    #[test]
    fn hash_is_content_sensitive() {
        let first = mutation("0.5");
        let second = mutation("0.6");
        assert_ne!(first.hash(), second.hash());
        assert_eq!(first.hash(), first.hash());
    }

    #[test]
    fn size_and_executable_guards() {
        assert!(mutation("0.5").validate().is_ok());
        assert!(mutation("#!/bin/sh").validate().is_err());
        assert!(mutation("subprocess.run").validate().is_err());
        assert!(mutation(&"x".repeat(MAX_MUTATION_BYTES + 1))
            .validate()
            .is_err());
    }
}
