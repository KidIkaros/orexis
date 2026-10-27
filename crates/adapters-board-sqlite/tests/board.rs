//! The board adapter against the contracts the fake encodes (M8-02):
//! edges gate readiness, the cap refuses visibly, transitions are
//! ordered, reopen preserves.

use futures::executor::block_on;
use orexis_adapters_board_sqlite::SqliteBoard;
use orexis_core::TaskStatus;
use orexis_ports::BoardStore;

fn board() -> (tempfile::TempDir, SqliteBoard) {
    let dir = tempfile::tempdir().expect("tempdir");
    let board = SqliteBoard::open(&dir.path().join("board.db")).expect("open");
    (dir, board)
}

#[test]
fn edges_gate_readiness() {
    let (_dir, board) = board();
    let first = block_on(board.create("first", "build", vec![])).expect("create");
    let second = block_on(board.create("second", "build", vec![first.id.clone()])).expect("create");

    let ready = block_on(board.ready()).expect("ready");
    assert_eq!(ready.len(), 1, "only the edgeless task is ready");

    block_on(board.admit(&first.id)).expect("admit");
    block_on(board.start(&first.id, "worker-1")).expect("start");
    block_on(board.complete(&first.id)).expect("complete");

    let ready = block_on(board.ready()).expect("ready");
    assert_eq!(
        ready
            .iter()
            .map(|task| task.id.as_str())
            .collect::<Vec<_>>(),
        vec![second.id.as_str()]
    );
}

#[test]
fn the_cap_refuses_visibly() {
    let dir = tempfile::tempdir().expect("tempdir");
    let board = SqliteBoard::open_with_cap(&dir.path().join("board.db"), 2).expect("open");
    block_on(board.create("a", "build", vec![])).expect("a");
    block_on(board.create("b", "build", vec![])).expect("b");
    let error = block_on(board.create("c", "build", vec![])).expect_err("full");
    assert!(error.0.contains("full"), "error: {error}");
}

#[test]
fn transitions_are_ordered_and_errors_name_the_state() {
    let (_dir, board) = board();
    let task = block_on(board.create("t", "build", vec![])).expect("create");
    let error = block_on(board.start(&task.id, "w")).expect_err("not admitted");
    assert!(error.0.contains("queued"), "error: {error}");
    block_on(board.admit(&task.id)).expect("admit");
    let running = block_on(board.start(&task.id, "worker-1")).expect("start");
    assert_eq!(running.status, TaskStatus::Running);
    assert_eq!(running.owner.as_deref(), Some("worker-1"));
    let done = block_on(board.complete(&task.id)).expect("complete");
    assert_eq!(done.status, TaskStatus::Done);
}

#[test]
fn unknown_edges_and_tasks_are_refused() {
    let (_dir, board) = board();
    let error = block_on(board.create("t", "build", vec!["no-such".to_string()]))
        .expect_err("unknown edge");
    assert!(error.0.contains("unknown task"), "error: {error}");
    assert!(block_on(board.get("no-such")).expect("get").is_none());
    let error = block_on(board.admit("no-such")).expect_err("no task");
    assert!(error.0.contains("no task"), "error: {error}");
}

#[test]
fn reopen_preserves_tasks() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("board.db");
    {
        let board = SqliteBoard::open(&path).expect("open");
        let task = block_on(board.create("kept", "build", vec![])).expect("create");
        block_on(board.admit(&task.id)).expect("admit");
    }
    let board = SqliteBoard::open(&path).expect("reopen");
    let tasks = block_on(board.list()).expect("list");
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].title, "kept");
    assert_eq!(tasks[0].status, TaskStatus::Admitted);
}
