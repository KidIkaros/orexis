//! Corridor: apply/revert, chain/signature verification, restart, export/import.

use std::collections::BTreeMap;
use std::sync::Arc;

use futures::executor::block_on;
use orexis_adapters_evolution_git::{import_jsonl, Ed25519Signer, GitCorridor};
use orexis_core::{CandidateRef, LedgerOp, Seal};
use orexis_ports::{EvolutionStore, Signer};

fn signer() -> Arc<dyn Signer> {
    Arc::new(Ed25519Signer::from_seed([7u8; 32]))
}

fn seal() -> Seal {
    Seal::compute(BTreeMap::from([
        ("permissions".to_string(), "standard".to_string()),
        ("sandbox".to_string(), "workspace".to_string()),
    ]))
}

fn candidate() -> CandidateRef {
    CandidateRef {
        id: "c1".to_string(),
        content_hash: "hash-c1".to_string(),
    }
}

#[test]
fn apply_and_revert_verify() {
    let dir = tempfile::tempdir().expect("tempdir");
    let corridor = GitCorridor::open(dir.path().join("evo"), signer()).expect("corridor");
    let applied = block_on(corridor.apply(candidate(), seal())).expect("apply");
    assert_eq!(applied.seq, 1);
    assert_eq!(applied.op, LedgerOp::Apply);
    block_on(corridor.verify()).expect("verify apply");

    let reverted = block_on(corridor.revert("c1", "regression")).expect("revert");
    assert_eq!(reverted.seq, 2);
    assert_eq!(reverted.op, LedgerOp::Revert);
    block_on(corridor.verify()).expect("verify revert");
    assert_eq!(block_on(corridor.entries()).expect("entries").len(), 2);
}

#[test]
fn tamper_breaks_verification() {
    let dir = tempfile::tempdir().expect("tempdir");
    let corridor = GitCorridor::open(dir.path().join("evo"), signer()).expect("corridor");
    block_on(corridor.apply(candidate(), seal())).expect("apply");
    let ledger = corridor.export_jsonl().expect("export");
    let tampered = ledger.replace("hash-c1", "hash-tampered");
    assert_ne!(ledger, tampered);
    std::fs::write(dir.path().join("evo/ledger.jsonl"), tampered).expect("tamper");
    assert!(block_on(corridor.verify()).is_err());
}

#[test]
fn restart_preserves_the_ledger() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().join("evo");
    {
        let corridor = GitCorridor::open(&root, signer()).expect("corridor");
        block_on(corridor.apply(candidate(), seal())).expect("apply");
    }
    let reopened = GitCorridor::open(&root, signer()).expect("reopen");
    assert_eq!(block_on(reopened.entries()).expect("entries").len(), 1);
    block_on(reopened.verify()).expect("verify");
}

#[test]
fn export_import_round_trips() {
    let dir = tempfile::tempdir().expect("tempdir");
    let source = GitCorridor::open(dir.path().join("a"), signer()).expect("source");
    block_on(source.apply(candidate(), seal())).expect("apply");
    let jsonl = source.export_jsonl().expect("export");
    let target = dir.path().join("b");
    block_on(import_jsonl(&target, signer(), &jsonl)).expect("import");
    let imported = GitCorridor::open(&target, signer()).expect("imported");
    assert_eq!(
        block_on(source.entries()).expect("source"),
        block_on(imported.entries()).expect("imported")
    );
    block_on(imported.verify()).expect("verify");
}
