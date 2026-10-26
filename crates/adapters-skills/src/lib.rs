//! Skill packages: manifests, hashes, trust tiers, and quarantine.

use std::fs;
use std::path::Path;

use orexis_ports::{
    PortError, PortResult, SkillContent, SkillStatus, SkillSummary, SkillTier, Skills,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};

#[derive(Debug, Deserialize)]
struct Manifest {
    name: String,
    version: String,
    tier: String,
    sha256: Option<String>,
    #[serde(default)]
    permissions: Vec<String>,
}

struct Skill {
    summary: SkillSummary,
    text: String,
}

pub struct SkillRegistry {
    skills: Vec<Skill>,
}

impl SkillRegistry {
    pub fn load(root: &Path) -> PortResult<Self> {
        Self::with_community_allowed(root, false)
    }

    pub fn with_community_allowed(root: &Path, allow_community: bool) -> PortResult<Self> {
        let mut skills = Vec::new();
        let entries = fs::read_dir(root)
            .map_err(|error| PortError(format!("skills {}: {error}", root.display())))?;
        for entry in entries {
            let dir = entry.map_err(|error| PortError(error.to_string()))?.path();
            if !dir.is_dir() {
                continue;
            }
            let manifest_path = dir.join("skill.yaml");
            if !manifest_path.exists() {
                continue;
            }
            let manifest_text = fs::read_to_string(&manifest_path).map_err(|error| {
                PortError(format!("manifest {}: {error}", manifest_path.display()))
            })?;
            let manifest: Manifest = serde_yaml::from_str(&manifest_text).map_err(|error| {
                PortError(format!("manifest {}: {error}", manifest_path.display()))
            })?;
            let text = fs::read_to_string(dir.join("SKILL.md")).unwrap_or_default();
            let hash = sha256_hex(&text);
            let tier = parse_tier(&manifest.tier);
            let status = status_for(tier, manifest.sha256.as_deref(), &hash, allow_community);
            skills.push(Skill {
                summary: SkillSummary {
                    name: manifest.name,
                    version: manifest.version,
                    tier,
                    status,
                    sha256: manifest.sha256,
                    permissions: manifest.permissions,
                },
                text,
            });
        }
        skills.sort_by(|left, right| left.summary.name.cmp(&right.summary.name));
        Ok(Self { skills })
    }
}

impl Skills for SkillRegistry {
    fn list(&self) -> Vec<SkillSummary> {
        self.skills
            .iter()
            .map(|skill| skill.summary.clone())
            .collect()
    }

    fn load(&self, name: &str) -> PortResult<SkillContent> {
        let skill = self
            .skills
            .iter()
            .find(|skill| skill.summary.name == name)
            .ok_or_else(|| PortError(format!("unknown skill {name}")))?;
        if skill.summary.status != SkillStatus::Allowed {
            return Err(PortError(format!(
                "skill {name} is {}",
                status_label(skill.summary.status)
            )));
        }
        Ok(SkillContent {
            name: skill.summary.name.clone(),
            text: skill.text.clone(),
        })
    }
}

fn parse_tier(value: &str) -> SkillTier {
    match value {
        "trusted" => SkillTier::Trusted,
        "verified" => SkillTier::Verified,
        "community" => SkillTier::Community,
        _ => SkillTier::Quarantined,
    }
}

fn status_for(
    tier: SkillTier,
    expected: Option<&str>,
    actual: &str,
    allow_community: bool,
) -> SkillStatus {
    if tier == SkillTier::Quarantined {
        return SkillStatus::Quarantined;
    }
    if tier == SkillTier::Community && !allow_community {
        return SkillStatus::CommunityBlocked;
    }
    match expected {
        Some(expected) if expected.eq_ignore_ascii_case(actual) => SkillStatus::Allowed,
        _ => SkillStatus::Quarantined,
    }
}

fn status_label(status: SkillStatus) -> &'static str {
    match status {
        SkillStatus::Allowed => "allowed",
        SkillStatus::CommunityBlocked => "community-blocked",
        SkillStatus::Quarantined => "quarantined",
    }
}

pub fn sha256_hex(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_skill(root: &Path, name: &str, tier: &str, hash: Option<&str>, text: &str) {
        let dir = root.join(name);
        fs::create_dir_all(&dir).expect("mkdir");
        let hash_line = hash
            .map(|hash| format!("sha256: {hash}\n"))
            .unwrap_or_default();
        fs::write(
            dir.join("skill.yaml"),
            format!("name: {name}\nversion: 1.0.0\ntier: {tier}\n{hash_line}permissions: [read]\n"),
        )
        .expect("manifest");
        fs::write(dir.join("SKILL.md"), text).expect("skill");
    }

    #[test]
    fn verified_skill_loads() {
        let dir = tempfile::tempdir().expect("tempdir");
        let hash = sha256_hex("hello");
        write_skill(dir.path(), "demo", "verified", Some(&hash), "hello");
        let registry = SkillRegistry::load(dir.path()).expect("registry");
        let listed = registry.list();
        assert_eq!(listed[0].status, SkillStatus::Allowed);
        assert_eq!(registry.load("demo").expect("load").text, "hello");
    }

    #[test]
    fn tampered_skill_is_quarantined() {
        let dir = tempfile::tempdir().expect("tempdir");
        let hash = sha256_hex("hello");
        write_skill(dir.path(), "demo", "verified", Some(&hash), "tampered");
        let registry = SkillRegistry::load(dir.path()).expect("registry");
        assert_eq!(registry.list()[0].status, SkillStatus::Quarantined);
        assert!(registry.load("demo").is_err());
    }

    #[test]
    fn community_is_blocked_by_default() {
        let dir = tempfile::tempdir().expect("tempdir");
        let hash = sha256_hex("hello");
        write_skill(dir.path(), "demo", "community", Some(&hash), "hello");
        let registry = SkillRegistry::load(dir.path()).expect("registry");
        assert_eq!(registry.list()[0].status, SkillStatus::CommunityBlocked);
        assert!(registry.load("demo").is_err());

        let allowed = SkillRegistry::with_community_allowed(dir.path(), true).expect("registry");
        assert_eq!(allowed.list()[0].status, SkillStatus::Allowed);
        assert!(allowed.load("demo").is_ok());
    }

    #[test]
    fn quarantined_tier_never_loads() {
        let dir = tempfile::tempdir().expect("tempdir");
        let hash = sha256_hex("hello");
        write_skill(dir.path(), "demo", "quarantined", Some(&hash), "hello");
        let registry = SkillRegistry::load(dir.path()).expect("registry");
        assert_eq!(registry.list()[0].status, SkillStatus::Quarantined);
        assert!(registry.load("demo").is_err());
    }

    #[test]
    fn missing_hash_is_quarantined() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_skill(dir.path(), "demo", "verified", None, "hello");
        let registry = SkillRegistry::load(dir.path()).expect("registry");
        assert_eq!(registry.list()[0].status, SkillStatus::Quarantined);
    }
}
