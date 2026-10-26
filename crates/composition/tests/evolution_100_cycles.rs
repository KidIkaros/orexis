//! 100 accelerated evolution cycles: propose, validate, apply, rollback.

use std::collections::BTreeMap;
use std::sync::Arc;

use futures::executor::block_on;
use orexis_adapters_evolution_git::{Ed25519Signer, GitCorridor, Quarantine};
use orexis_core::{LedgerOp, Mutation, MutationKind, Seal, Tier, ValidationReport};
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

fn context(index: usize) -> ProposalContext {
    ProposalContext {
        session_id: format!("s{index}"),
        day: index as u64 + 1,
        boundary: true,
    }
}

#[test]
fn hundred_cycles_are_durable() {
    let dir = tempfile::tempdir().expect("tempdir");
    let queue = Quarantine::open(dir.path().join("quarantine")).expect("queue");
    let corridor = GitCorridor::open(dir.path().join("corridor"), signer()).expect("corridor");
    let seal = seal();

    let executable = Mutation {
        kind: MutationKind::SkillContent,
        target: "skills/bad".to_string(),
        base_hash: "base".to_string(),
        content: "#!/bin/sh".to_string(),
    };
    assert!(block_on(queue.propose(executable, context(999), "growth")).is_err());
    assert!(block_on(queue.list()).expect("list").is_empty());

    let mut rollbacks = 0;
    for index in 0..100 {
        let mutation = Mutation {
            kind: MutationKind::SkillContent,
            target: "skills/demo".to_string(),
            base_hash: "base".to_string(),
            content: format!("skill content {index}"),
        };
        let proposal =
            block_on(queue.propose(mutation, context(index), "growth")).expect("proposal");
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
                signed: false,
                validation,
                seal: seal.clone(),
            },
        ))
        .expect("apply");
        assert_eq!(entry.seal_hash, seal.hash);
        block_on(corridor.verify()).expect("verify apply");

        if index % 10 == 0 {
            block_on(rollback(&corridor, &candidate_id, "scheduled")).expect("rollback");
            rollbacks += 1;
            block_on(corridor.verify()).expect("verify rollback");
        }
    }

    let entries = block_on(corridor.entries()).expect("entries");
    assert_eq!(entries.len(), 100 + rollbacks);
    assert!(entries
        .iter()
        .filter(|entry| entry.op == LedgerOp::Apply)
        .all(|entry| entry.seal_hash == seal.hash));
    block_on(corridor.verify()).expect("final verify");
    assert_eq!(block_on(queue.list()).expect("list").len(), 100);
}
