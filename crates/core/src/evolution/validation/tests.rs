use super::*;

use crate::evolution::{CandidateManifest, Mutation, MutationKind};
use std::collections::BTreeMap;

fn seal() -> Seal {
    Seal::compute(BTreeMap::from([(
        "permissions".to_string(),
        "standard".to_string(),
    )]))
}

fn candidate(target: &str, content: &str) -> Candidate {
    let mutation = Mutation {
        kind: MutationKind::GeneParameter,
        target: target.to_string(),
        base_hash: "base".to_string(),
        content: content.to_string(),
    };
    Candidate {
        manifest: CandidateManifest {
            id: "c1".to_string(),
            kind: mutation.kind,
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

fn inputs(candidate: Candidate) -> ValidationInputs {
    ValidationInputs {
        candidate,
        candidate_seal: seal(),
        current_seal: seal(),
        deterministic: vec![DeterministicTest {
            name: "fixture".to_string(),
            passed: true,
        }],
        behavior: BehavioralResult {
            baseline: 0.8,
            candidate: 0.8,
        },
        soak_hours: 24,
        human_approved: true,
    }
}

#[test]
fn full_ladder_passes() {
    let report = validate(
        &inputs(candidate("drives.curiosity", "0.6")),
        &ValidationPolicy::default(),
    );
    assert!(report.passed);
    assert_eq!(report.stages.len(), 7);
}

#[test]
fn sealed_target_and_seal_change_fail() {
    let report = validate(
        &inputs(candidate("permissions.mode", "allow")),
        &ValidationPolicy::default(),
    );
    assert!(!report.passed);
    assert!(!report.stages[2].passed);

    let mut changed = inputs(candidate("drives.curiosity", "0.6"));
    changed.current_seal = Seal::compute(BTreeMap::from([(
        "permissions".to_string(),
        "plan".to_string(),
    )]));
    let report = validate(&changed, &ValidationPolicy::default());
    assert!(!report.passed);
    assert!(!report.stages[1].passed);
}

#[test]
fn regression_and_pending_human_fail() {
    let mut regressed = inputs(candidate("drives.curiosity", "0.6"));
    regressed.behavior.candidate = 0.5;
    let report = validate(&regressed, &ValidationPolicy::default());
    assert!(!report.passed);
    assert!(!report.stages[4].passed);

    let mut pending = inputs(candidate("drives.curiosity", "0.6"));
    pending.human_approved = false;
    let report = validate(&pending, &ValidationPolicy::default());
    assert!(!report.passed);
    assert!(!report.stages[6].passed);
}

#[test]
fn executable_and_missing_tests_fail() {
    let report = validate(
        &inputs(candidate("drives.curiosity", "#!/bin/sh")),
        &ValidationPolicy::default(),
    );
    assert!(!report.passed);
    assert!(!report.stages[0].passed);

    let mut empty = inputs(candidate("drives.curiosity", "0.6"));
    empty.deterministic.clear();
    let report = validate(&empty, &ValidationPolicy::default());
    assert!(!report.passed);
    assert!(!report.stages[3].passed);
}
