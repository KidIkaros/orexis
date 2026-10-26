//! Seal: a hash of the protected configuration a mutation may not change.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Seal {
    pub hash: String,
    pub fields: BTreeMap<String, String>,
}

impl Seal {
    pub fn compute(fields: BTreeMap<String, String>) -> Self {
        let canonical = fields
            .iter()
            .map(|(key, value)| format!("{key}\u{1f}{value}"))
            .collect::<Vec<_>>()
            .join("\u{1e}");
        let hash = blake3::hash(canonical.as_bytes()).to_hex().to_string();
        Self { hash, fields }
    }

    pub fn changed(&self, other: &Self) -> bool {
        self.hash != other.hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fields(value: &str) -> BTreeMap<String, String> {
        BTreeMap::from([
            ("permissions".to_string(), value.to_string()),
            ("sandbox".to_string(), "workspace".to_string()),
        ])
    }

    #[test]
    fn seal_is_deterministic_and_change_sensitive() {
        let first = Seal::compute(fields("standard"));
        let second = Seal::compute(fields("standard"));
        let changed = Seal::compute(fields("plan"));
        assert_eq!(first, second);
        assert!(first.changed(&changed));
        assert!(!first.changed(&second));
    }
}
