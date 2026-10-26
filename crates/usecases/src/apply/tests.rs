use super::*;

use std::collections::BTreeMap;
use std::sync::Mutex;

use async_trait::async_trait;
use futures::executor::block_on;
use orexis_core::{
    Candidate, CandidateManifest, LedgerOp, Mutation, MutationKind, StageResult, ValidationStage,
};
use orexis_ports::{Proposal, ProposalContext};

struct FakeQueue {
    candidate: Candidate,
}

#[async_trait]
impl MutationQueue for FakeQueue {
    async fn propose(
        &self,
        _mutation: Mutation,
        _context: ProposalContext,
        _source: &str,
    ) -> PortResult<Proposal> {
        Err(PortError("unused".into()))
    }

    async fn list(&self) -> PortResult<Vec<CandidateManifest>> {
        Ok(vec![self.candidate.manifest.clone()])
    }

    async fn get(&self, _id: &str) -> PortResult<Option<CandidateManifest>> {
        Ok(Some(self.candidate.manifest.clone()))
    }

    async fn candidate(&self, _id: &str) -> PortResult<Option<Candidate>> {
        Ok(Some(self.candidate.clone()))
    }
}

#[derive(Default)]
struct FakeCorridor {
    applied: Mutex<Vec<String>>,
    reverted: Mutex<Vec<String>>,
}

#[async_trait]
impl EvolutionStore for FakeCorridor {
    async fn apply(&self, candidate: CandidateRef, seal: Seal) -> PortResult<LedgerEntry> {
        self.applied
            .lock()
            .expect("applied")
            .push(candidate.id.clone());
        Ok(entry(candidate.id, LedgerOp::Apply, &seal.hash))
    }

    async fn revert(&self, candidate_id: &str, _reason: &str) -> PortResult<LedgerEntry> {
        self.reverted
            .lock()
            .expect("reverted")
            .push(candidate_id.to_string());
        Ok(entry(candidate_id.to_string(), LedgerOp::Revert, "seal"))
    }

    async fn entries(&self) -> PortResult<Vec<LedgerEntry>> {
        Ok(Vec::new())
    }

    async fn verify(&self) -> PortResult<()> {
        Ok(())
    }

    fn export_jsonl(&self) -> PortResult<String> {
        Ok(String::new())
    }
}

fn entry(id: String, op: LedgerOp, seal: &str) -> LedgerEntry {
    LedgerEntry {
        seq: 1,
        op,
        candidate_id: id,
        candidate_hash: "hash".to_string(),
        seal_hash: seal.to_string(),
        git_commit: "commit".to_string(),
        prev_hash: "genesis".to_string(),
        timestamp: 1,
        signature: "sig".to_string(),
    }
}

fn candidate(kind: MutationKind) -> Candidate {
    let mutation = Mutation {
        kind,
        target: "skills/demo".to_string(),
        base_hash: "base".to_string(),
        content: "hello".to_string(),
    };
    Candidate {
        manifest: CandidateManifest {
            id: "c1".to_string(),
            kind,
            target: mutation.target.clone(),
            base_hash: mutation.base_hash.clone(),
            content_hash: mutation.hash(),
            size: mutation.size(),
            session_id: "s1".to_string(),
            day: 1,
            created_at: 1,
            source: "growth".to_string(),
        },
        mutation,
    }
}

fn validation(passed: bool) -> ValidationReport {
    ValidationReport {
        candidate_id: "c1".to_string(),
        stages: vec![StageResult {
            stage: ValidationStage::Schema,
            passed,
            detail: String::new(),
        }],
        passed,
    }
}

fn request(tier: Tier, human: bool, signed: bool) -> ApplyRequest {
    ApplyRequest {
        candidate_id: "c1".to_string(),
        tier,
        human_approved: human,
        signed,
        validation: validation(true),
        seal: Seal::compute(BTreeMap::new()),
    }
}

#[test]
fn off_refuses_and_skills_applies_skill_content() {
    let queue = FakeQueue {
        candidate: candidate(MutationKind::SkillContent),
    };
    let corridor = FakeCorridor::default();
    assert!(block_on(apply_candidate(
        &queue,
        &corridor,
        request(Tier::Off, false, false)
    ))
    .is_err());

    let applied = block_on(apply_candidate(
        &queue,
        &corridor,
        request(Tier::Skills, false, false),
    ))
    .expect("skills apply");
    assert_eq!(applied.op, LedgerOp::Apply);
}

#[test]
fn skills_cannot_apply_genes_and_core_needs_signature() {
    let queue = FakeQueue {
        candidate: candidate(MutationKind::GeneParameter),
    };
    let corridor = FakeCorridor::default();
    assert!(block_on(apply_candidate(
        &queue,
        &corridor,
        request(Tier::Skills, true, true)
    ))
    .is_err());
    assert!(block_on(apply_candidate(
        &queue,
        &corridor,
        request(Tier::Core, true, false)
    ))
    .is_err());
    assert!(block_on(apply_candidate(
        &queue,
        &corridor,
        request(Tier::Core, true, true)
    ))
    .is_ok());
}

#[test]
fn invalid_validation_is_refused() {
    let queue = FakeQueue {
        candidate: candidate(MutationKind::SkillContent),
    };
    let corridor = FakeCorridor::default();
    let mut request = request(Tier::Skills, false, false);
    request.validation = validation(false);
    assert!(block_on(apply_candidate(&queue, &corridor, request)).is_err());
}

#[test]
fn rollback_calls_revert() {
    let corridor = FakeCorridor::default();
    let entry = block_on(rollback(&corridor, "c1", "regression")).expect("rollback");
    assert_eq!(entry.op, LedgerOp::Revert);
    assert_eq!(
        corridor.reverted.lock().expect("reverted").as_slice(),
        ["c1"]
    );
}
