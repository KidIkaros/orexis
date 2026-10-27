//! The blackboard adapter against the contracts the fake encodes (M8-04):
//! jailed paths, capped artifacts, expiring + stealable leases, reopen
//! preserves leases.

use futures::executor::block_on;
use orexis_adapters_artifacts_fs::FsBlackboard;
use orexis_ports::ArtifactStore;

#[test]
fn artifacts_round_trip_and_are_jailed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let blackboard = FsBlackboard::open(dir.path().join("blackboard")).expect("open");
    block_on(blackboard.put("diffs/turn-1.patch", b"a diff")).expect("put");
    let read = block_on(blackboard.get("diffs/turn-1.patch")).expect("get");
    assert_eq!(read.as_deref(), Some(b"a diff".as_slice()));
    assert!(block_on(blackboard.get("missing")).expect("get").is_none());
    assert!(block_on(blackboard.put("../escape", b"x")).is_err());
    assert!(block_on(blackboard.put("/absolute", b"x")).is_err());
    assert!(block_on(blackboard.put("", b"x")).is_err());
    let names = block_on(blackboard.list()).expect("list");
    assert_eq!(names, vec!["diffs/turn-1.patch"]);
}

#[test]
fn oversized_artifacts_are_refused() {
    let dir = tempfile::tempdir().expect("tempdir");
    let blackboard = FsBlackboard::open(dir.path().join("blackboard")).expect("open");
    let huge = vec![0u8; 9 * 1024 * 1024];
    let error = block_on(blackboard.put("huge", &huge)).expect_err("capped");
    assert!(error.0.contains("cap"), "error: {error}");
}

#[test]
fn leases_expire_and_are_stealable() {
    let dir = tempfile::tempdir().expect("tempdir");
    let blackboard = FsBlackboard::open(dir.path().join("blackboard")).expect("open");
    block_on(blackboard.lease("src/main.rs", "w1", 3600)).expect("lease");
    let error = block_on(blackboard.lease("src/main.rs", "w2", 3600)).expect_err("leased");
    assert!(error.0.contains("leased by"), "error: {error}");
    block_on(blackboard.lease("src/lib.rs", "w1", 0)).expect("expired ttl");
    let stolen = block_on(blackboard.steal("src/lib.rs", "w2")).expect("steal");
    assert!(stolen.is_some(), "an expired lease is stealable");
    let none = block_on(blackboard.steal("src/main.rs", "w2")).expect("steal");
    assert!(none.is_none(), "a live lease is not stealable");
}

#[test]
fn reopen_preserves_leases() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().join("blackboard");
    {
        let blackboard = FsBlackboard::open(&root).expect("open");
        block_on(blackboard.lease("src/main.rs", "w1", 3600)).expect("lease");
    }
    let blackboard = FsBlackboard::open(&root).expect("reopen");
    let lease = block_on(blackboard.lease_of("src/main.rs"))
        .expect("lease_of")
        .expect("preserved");
    assert_eq!(lease.owner, "w1");
}
