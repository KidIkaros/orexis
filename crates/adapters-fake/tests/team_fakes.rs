//! The team fakes encode the contracts the sqlite adapters must keep
//! (M8-01): edges gate readiness, caps refuse visibly, leases expire and
//! are stealable, the bus is seq-ordered and filtered.

use futures::executor::block_on;
use orexis_adapters_fake::{FakeBlackboard, FakeBoard, FakeBus};
use orexis_core::TaskStatus;
use orexis_ports::{ArtifactStore, BoardStore, MessageStore};

#[test]
fn edges_gate_readiness() {
    let board = FakeBoard::new(256);
    let first = block_on(board.create("first", "build", vec![])).expect("create");
    let second = block_on(board.create("second", "build", vec![first.id.clone()])).expect("create");

    // The dependent task is not ready while its edge is queued.
    let ready = block_on(board.ready()).expect("ready");
    assert_eq!(
        ready
            .iter()
            .map(|task| task.id.as_str())
            .collect::<Vec<_>>(),
        vec!["task-1"]
    );

    block_on(board.admit(&first.id)).expect("admit");
    block_on(board.start(&first.id, "worker-1")).expect("start");
    block_on(board.complete(&first.id)).expect("complete");

    // Now the dependent is ready.
    let ready = block_on(board.ready()).expect("ready");
    assert_eq!(
        ready
            .iter()
            .map(|task| task.id.as_str())
            .collect::<Vec<_>>(),
        vec!["task-2"]
    );
    let _ = second;
}

#[test]
fn the_cap_refuses_visibly() {
    let board = FakeBoard::new(2);
    block_on(board.create("a", "build", vec![])).expect("a");
    block_on(board.create("b", "build", vec![])).expect("b");
    let error = block_on(board.create("c", "build", vec![])).expect_err("full");
    assert!(error.0.contains("full"), "error: {error}");
}

#[test]
fn status_transitions_are_ordered() {
    let board = FakeBoard::new(256);
    let task = block_on(board.create("t", "build", vec![])).expect("create");
    // A worker cannot start an unadmitted task.
    let error = block_on(board.start(&task.id, "w")).expect_err("not admitted");
    assert!(error.0.contains("not admitted"), "error: {error}");
    block_on(board.admit(&task.id)).expect("admit");
    let running = block_on(board.start(&task.id, "worker-1")).expect("start");
    assert_eq!(running.status, TaskStatus::Running);
    assert_eq!(running.owner.as_deref(), Some("worker-1"));
    let done = block_on(board.complete(&task.id)).expect("complete");
    assert_eq!(done.status, TaskStatus::Done);
}

#[test]
fn the_bus_is_seq_ordered_and_filtered() {
    let bus = FakeBus::new();
    block_on(bus.post("w1", None, "room hello")).expect("room");
    block_on(bus.post("w1", Some("w2"), "direct to w2")).expect("direct");

    // A worker sees the room plus its own direct messages.
    let for_w2 = block_on(bus.read_since(0, Some("w2"))).expect("read");
    assert_eq!(for_w2.len(), 2);
    // Another worker sees only the room.
    let for_w3 = block_on(bus.read_since(0, Some("w3"))).expect("read");
    assert_eq!(for_w3.len(), 1);
    assert_eq!(for_w3[0].text, "room hello");
    // The supervisor sees everything since a seq.
    let all = block_on(bus.read_since(1, None)).expect("read");
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].seq, 2);
}

#[test]
fn leases_expire_and_are_stealable() {
    let blackboard = FakeBlackboard::new();
    block_on(blackboard.lease("src/main.rs", "w1", 3600)).expect("lease");
    // A live lease held by another owner is refused.
    let error = block_on(blackboard.lease("src/main.rs", "w2", 3600)).expect_err("leased");
    assert!(error.0.contains("leased by"), "error: {error}");
    // An expired lease is stealable.
    block_on(blackboard.lease("src/lib.rs", "w1", 0)).expect("expired ttl");
    let stolen = block_on(blackboard.steal("src/lib.rs", "w2")).expect("steal");
    assert!(stolen.is_some(), "an expired lease is stealable");
    // A live lease is not stealable.
    let none = block_on(blackboard.steal("src/main.rs", "w2")).expect("steal");
    assert!(none.is_none(), "a live lease is not stealable");
}

#[test]
fn artifacts_are_jailed_and_capped_names() {
    let blackboard = FakeBlackboard::new();
    block_on(blackboard.put("diffs/turn-1.patch", b"a diff")).expect("put");
    let read = block_on(blackboard.get("diffs/turn-1.patch")).expect("get");
    assert_eq!(read.as_deref(), Some(b"a diff".as_slice()));
    assert!(block_on(blackboard.put("../escape", b"x")).is_err());
    assert!(block_on(blackboard.put("/absolute", b"x")).is_err());
}
