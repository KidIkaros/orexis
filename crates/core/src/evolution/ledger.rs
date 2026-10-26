//! Evolution ledger entries.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LedgerOp {
    Apply,
    Revert,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateRef {
    pub id: String,
    pub content_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub seq: u64,
    pub op: LedgerOp,
    pub candidate_id: String,
    pub candidate_hash: String,
    pub seal_hash: String,
    pub git_commit: String,
    pub prev_hash: String,
    pub timestamp: u64,
    pub signature: String,
}

impl LedgerEntry {
    pub fn unsigned_hash(&self) -> String {
        let mut copy = self.clone();
        copy.signature.clear();
        let bytes = serde_json::to_vec(&copy).unwrap_or_default();
        blake3::hash(&bytes).to_hex().to_string()
    }

    pub fn signed_hash(&self) -> String {
        let bytes = serde_json::to_vec(self).unwrap_or_default();
        blake3::hash(&bytes).to_hex().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry() -> LedgerEntry {
        LedgerEntry {
            seq: 1,
            op: LedgerOp::Apply,
            candidate_id: "c1".to_string(),
            candidate_hash: "abc".to_string(),
            seal_hash: "seal".to_string(),
            git_commit: "deadbeef".to_string(),
            prev_hash: "genesis".to_string(),
            timestamp: 1,
            signature: String::new(),
        }
    }

    #[test]
    fn signatures_change_the_hash() {
        let mut entry = entry();
        let unsigned = entry.unsigned_hash();
        entry.signature = "00".to_string();
        assert_eq!(entry.unsigned_hash(), unsigned);
        assert_eq!(entry.unsigned_hash(), entry.unsigned_hash());
        assert_ne!(entry.signed_hash(), unsigned);
    }
}
