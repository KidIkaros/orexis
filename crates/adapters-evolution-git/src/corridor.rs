//! Git-backed evolution corridor and signed ledger.

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use orexis_core::{CandidateRef, LedgerEntry, LedgerOp, Seal};
use orexis_ports::{EvolutionStore, PortError, PortResult, Signer};

use crate::git::{git_output, hex, hex_decode, run_git};

pub struct GitCorridor {
    root: PathBuf,
    ledger: PathBuf,
    signer: Arc<dyn Signer>,
}

impl GitCorridor {
    pub fn open(root: impl Into<PathBuf>, signer: Arc<dyn Signer>) -> PortResult<Self> {
        let root = root.into();
        fs::create_dir_all(&root)
            .map_err(|error| PortError(format!("corridor {}: {error}", root.display())))?;
        if !root.join(".git").exists() {
            run_git(&root, &["init", "-q"])?;
        }
        let ledger = root.join("ledger.jsonl");
        if !ledger.exists() {
            fs::write(&ledger, "")
                .map_err(|error| PortError(format!("ledger {}: {error}", ledger.display())))?;
        }
        Ok(Self {
            root,
            ledger,
            signer,
        })
    }

    fn entries_sync(&self) -> PortResult<Vec<LedgerEntry>> {
        let text = fs::read_to_string(&self.ledger)
            .map_err(|error| PortError(format!("ledger {}: {error}", self.ledger.display())))?;
        text.lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| serde_json::from_str(line).map_err(|error| PortError(error.to_string())))
            .collect()
    }

    fn append(&self, entry: &LedgerEntry) -> PortResult<()> {
        let mut line =
            serde_json::to_string(entry).map_err(|error| PortError(error.to_string()))?;
        line.push('\n');
        use std::io::Write;
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.ledger)
            .map_err(|error| PortError(format!("ledger {}: {error}", self.ledger.display())))?;
        file.write_all(line.as_bytes())
            .map_err(|error| PortError(error.to_string()))
    }

    fn commit_paths(&self, paths: &[&str], message: &str) -> PortResult<String> {
        let mut add = vec!["add", "--"];
        add.extend_from_slice(paths);
        run_git(&self.root, &add)?;
        run_git(
            &self.root,
            &[
                "-c",
                "user.name=orexis",
                "-c",
                "user.email=orexis@localhost",
                "commit",
                "-q",
                "-m",
                message,
            ],
        )?;
        Ok(git_output(&self.root, &["rev-parse", "HEAD"])?
            .trim()
            .to_string())
    }

    fn commit(&self, message: &str) -> PortResult<String> {
        run_git(&self.root, &["add", "-A"])?;
        run_git(
            &self.root,
            &[
                "-c",
                "user.name=orexis",
                "-c",
                "user.email=orexis@localhost",
                "commit",
                "-q",
                "-m",
                message,
            ],
        )?;
        Ok(git_output(&self.root, &["rev-parse", "HEAD"])?
            .trim()
            .to_string())
    }

    fn next(
        &self,
        op: LedgerOp,
        candidate_id: &str,
        candidate_hash: &str,
        seal_hash: &str,
        git_commit: &str,
    ) -> PortResult<LedgerEntry> {
        let entries = self.entries_sync()?;
        let prev_hash = entries
            .last()
            .map(LedgerEntry::signed_hash)
            .unwrap_or_else(|| "genesis".to_string());
        Ok(LedgerEntry {
            seq: entries.len() as u64 + 1,
            op,
            candidate_id: candidate_id.to_string(),
            candidate_hash: candidate_hash.to_string(),
            seal_hash: seal_hash.to_string(),
            git_commit: git_commit.to_string(),
            prev_hash,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            signature: String::new(),
        })
    }
}

#[async_trait]
impl EvolutionStore for GitCorridor {
    async fn apply(&self, candidate: CandidateRef, seal: Seal) -> PortResult<LedgerEntry> {
        if candidate.id.trim().is_empty() || candidate.content_hash.trim().is_empty() {
            return Err(PortError("candidate id and hash are required".into()));
        }
        let dir = self.root.join("candidates");
        fs::create_dir_all(&dir)
            .map_err(|error| PortError(format!("candidates {}: {error}", dir.display())))?;
        let payload = serde_json::json!({ "candidate": candidate, "seal": seal });
        fs::write(
            dir.join(format!("{}.json", candidate.id)),
            serde_json::to_vec_pretty(&payload).map_err(|error| PortError(error.to_string()))?,
        )
        .map_err(|error| PortError(error.to_string()))?;
        let candidate_path = format!("candidates/{}.json", candidate.id);
        let candidate_commit =
            self.commit_paths(&[&candidate_path], &format!("candidate {}", candidate.id))?;

        let mut entry = self.next(
            LedgerOp::Apply,
            &candidate.id,
            &candidate.content_hash,
            &seal.hash,
            &candidate_commit,
        )?;
        let hash = entry.unsigned_hash();
        let signature = self.signer.sign(hash.as_bytes())?;
        entry.signature = hex(&signature);
        self.append(&entry)?;
        self.commit(&format!("ledger apply {}", entry.candidate_id))?;
        Ok(entry)
    }

    async fn revert(&self, candidate_id: &str, reason: &str) -> PortResult<LedgerEntry> {
        let entries = self.entries_sync()?;
        let applied = entries
            .iter()
            .rev()
            .find(|entry| entry.op == LedgerOp::Apply && entry.candidate_id == candidate_id)
            .ok_or_else(|| PortError(format!("no apply for candidate {candidate_id}")))?;
        let already_reverted = entries.iter().any(|entry| {
            entry.op == LedgerOp::Revert
                && entry.candidate_id == candidate_id
                && entry.seq > applied.seq
        });
        if already_reverted {
            return Err(PortError(format!(
                "candidate {candidate_id} already reverted"
            )));
        }
        run_git(&self.root, &["revert", "--no-commit", &applied.git_commit])?;
        let revert_commit = self.commit(&format!("revert {candidate_id}: {reason}"))?;
        let mut entry = self.next(
            LedgerOp::Revert,
            candidate_id,
            &applied.candidate_hash,
            &applied.seal_hash,
            &revert_commit,
        )?;
        let hash = entry.unsigned_hash();
        let signature = self.signer.sign(hash.as_bytes())?;
        entry.signature = hex(&signature);
        self.append(&entry)?;
        self.commit(&format!("ledger revert {candidate_id}"))?;
        Ok(entry)
    }

    async fn entries(&self) -> PortResult<Vec<LedgerEntry>> {
        self.entries_sync()
    }

    async fn verify(&self) -> PortResult<()> {
        let entries = self.entries_sync()?;
        let mut prev = "genesis".to_string();
        for (index, entry) in entries.iter().enumerate() {
            if entry.seq != index as u64 + 1 {
                return Err(PortError(format!("ledger seq gap at {}", entry.seq)));
            }
            if entry.prev_hash != prev {
                return Err(PortError(format!("ledger chain broken at {}", entry.seq)));
            }
            let signature = hex_decode(&entry.signature)?;
            if !self
                .signer
                .verify(entry.unsigned_hash().as_bytes(), &signature)
            {
                return Err(PortError(format!("bad signature at {}", entry.seq)));
            }
            prev = entry.signed_hash();
        }
        Ok(())
    }

    fn export_jsonl(&self) -> PortResult<String> {
        fs::read_to_string(&self.ledger)
            .map_err(|error| PortError(format!("ledger {}: {error}", self.ledger.display())))
    }
}

pub async fn import_jsonl(
    root: impl Into<PathBuf>,
    signer: Arc<dyn Signer>,
    jsonl: &str,
) -> PortResult<()> {
    let corridor = GitCorridor::open(root, signer)?;
    fs::write(&corridor.ledger, jsonl)
        .map_err(|error| PortError(format!("ledger import: {error}")))?;
    corridor.verify().await?;
    corridor.commit("import corridor")?;
    Ok(())
}
