//! Quarantine: propose, dedup, cadence, boundary, executable guard.

use futures::executor::block_on;
use orexis_adapters_evolution_git::Quarantine;
use orexis_core::{Mutation, MutationKind};
use orexis_ports::{MutationQueue, ProposalContext};

fn mutation(content: &str) -> Mutation {
    Mutation {
        kind: MutationKind::GeneParameter,
        target: "drives.curiosity".to_string(),
        base_hash: "base".to_string(),
        content: content.to_string(),
    }
}

fn context(session: &str, day: u64, boundary: bool) -> ProposalContext {
    ProposalContext {
        session_id: session.to_string(),
        day,
        boundary,
    }
}

#[test]
fn propose_list_and_get() {
    let dir = tempfile::tempdir().expect("tempdir");
    let queue = Quarantine::open(dir.path()).expect("queue");
    let proposal = block_on(queue.propose(mutation("0.6"), context("s1", 1, true), "growth"))
        .expect("proposal");
    assert!(!proposal.deduplicated);
    assert_eq!(block_on(queue.list()).expect("list").len(), 1);
    assert_eq!(
        block_on(queue.get(&proposal.manifest.id))
            .expect("get")
            .expect("manifest")
            .content_hash,
        proposal.manifest.content_hash
    );
}

#[test]
fn duplicate_content_is_deduplicated() {
    let dir = tempfile::tempdir().expect("tempdir");
    let queue = Quarantine::open(dir.path()).expect("queue");
    let first =
        block_on(queue.propose(mutation("0.6"), context("s1", 1, true), "growth")).expect("first");
    let second =
        block_on(queue.propose(mutation("0.6"), context("s2", 2, true), "growth")).expect("second");
    assert!(second.deduplicated);
    assert_eq!(first.manifest.id, second.manifest.id);
    assert_eq!(block_on(queue.list()).expect("list").len(), 1);
}

#[test]
fn cadence_is_one_per_session_and_day() {
    let dir = tempfile::tempdir().expect("tempdir");
    let queue = Quarantine::open(dir.path()).expect("queue");
    block_on(queue.propose(mutation("0.6"), context("s1", 1, true), "growth")).expect("first");
    assert!(block_on(queue.propose(mutation("0.7"), context("s1", 2, true), "growth")).is_err());
    assert!(block_on(queue.propose(mutation("0.8"), context("s2", 1, true), "growth")).is_err());
}

#[test]
fn boundary_and_executable_guards() {
    let dir = tempfile::tempdir().expect("tempdir");
    let queue = Quarantine::open(dir.path()).expect("queue");
    assert!(block_on(queue.propose(mutation("0.6"), context("s1", 1, false), "growth")).is_err());
    assert!(
        block_on(queue.propose(mutation("#!/bin/sh"), context("s2", 2, true), "growth")).is_err()
    );
}
