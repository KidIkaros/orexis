//! End-to-end: propose in quarantine, apply under the skills tier, roll back.

use std::collections::BTreeMap;
use std::sync::Arc;

use futures::executor::block_on;
use orexis_adapters_evolution_git::{Ed25519Signer, GitCorridor, Quarantine};
use orexis_core::{Mutation, MutationKind, Seal, Tier, ValidationReport};
use orexis_ports::{EvolutionStore, MutationQueue, ProposalContext, Signer};
use orexis_usecases::{apply_candidate, rollback, ApplyRequest};

fn signer() -> Arc<dyn Signer> {
    Arc::new(Ed25519Signer::from_seed([7u8; 32]))
}

fn seal() -> Seal {
    Seal::compute(BTreeMap::from([(
        "permissions".to_string(),
        "standard".to_string(),
    )]))
}

#[test]
fn skills_candidate_applies_and_rolls_back() {
    let dir = tempfile::tempdir().expect("tempdir");
    let queue = Quarantine::open(dir.path().join("quarantine")).expect("queue");
    let corridor = GitCorridor::open(dir.path().join("corridor"), signer()).expect("corridor");

    let mutation = Mutation {
        kind: MutationKind::SkillContent,
        target: "skills/demo".to_string(),
        base_hash: "base".to_string(),
        content: "a markdown skill".to_string(),
    };
    let proposal = block_on(queue.propose(
        mutation,
        ProposalContext {
            session_id: "s1".to_string(),
            day: 1,
            boundary: true,
        },
        "growth",
    ))
    .expect("proposal");
    let candidate_id = proposal.manifest.id;

    let validation = ValidationReport {
        candidate_id: candidate_id.clone(),
        stages: Vec::new(),
        passed: true,
    };
    let entry = block_on(apply_candidate(
        &queue,
        &corridor,
        ApplyRequest {
            candidate_id: candidate_id.clone(),
            tier: Tier::Skills,
            human_approved: false,
            candidate_signature: None,
            signer: None,
            validation,
            seal: seal(),
        },
    ))
    .expect("apply");
    assert_eq!(entry.candidate_id, candidate_id);
    block_on(corridor.verify()).expect("verify apply");

    let revert = block_on(rollback(&corridor, &candidate_id, "regression")).expect("rollback");
    assert_eq!(revert.candidate_id, candidate_id);
    block_on(corridor.verify()).expect("verify rollback");
    assert_eq!(block_on(corridor.entries()).expect("entries").len(), 2);
}

#[test]
fn core_candidate_applies_with_a_real_signature() {
    let dir = tempfile::tempdir().expect("tempdir");
    let queue = Quarantine::open(dir.path().join("quarantine")).expect("queue");
    let corridor = GitCorridor::open(dir.path().join("corridor"), signer()).expect("corridor");

    let mutation = Mutation {
        kind: MutationKind::GeneParameter,
        target: "drives.curiosity".to_string(),
        base_hash: "base".to_string(),
        content: "0.6".to_string(),
    };
    let proposal = block_on(queue.propose(
        mutation,
        ProposalContext {
            session_id: "s1".to_string(),
            day: 1,
            boundary: true,
        },
        "growth",
    ))
    .expect("proposal");
    let content_hash = proposal.manifest.content_hash.clone();
    let signature = signer().sign(content_hash.as_bytes()).expect("sign");

    let validation = ValidationReport {
        candidate_id: proposal.manifest.id.clone(),
        stages: Vec::new(),
        passed: true,
    };
    let entry = block_on(apply_candidate(
        &queue,
        &corridor,
        ApplyRequest {
            candidate_id: proposal.manifest.id.clone(),
            tier: Tier::Core,
            human_approved: true,
            candidate_signature: Some(signature),
            signer: Some(signer()),
            validation,
            seal: seal(),
        },
    ))
    .expect("signed core apply");
    assert_eq!(entry.candidate_id, proposal.manifest.id);
    block_on(corridor.verify()).expect("verify apply");
}
