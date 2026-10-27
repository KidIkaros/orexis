//! The scheduler (M8-06, ADR-0020, invariant 17): admits board tasks
//! under the provider's capacity — asked, never guessed — the roster
//! cap, and flat spawn depth. Exhaustion produces a visible queue: the
//! tasks stay `queued` on the board, readable by anyone.

use orexis_core::TaskStatus;
use orexis_ports::{BoardStore, PortResult};

/// The roster cap (§9: 16; a team may tighten, never exceed).
pub const ROSTER_CAP: usize = 16;

/// Admit ready tasks up to the team's slots: the provider's capacity
/// asked, the roster cap applied, minus the tasks already running. FIFO
/// by board seq. Returns the tasks admitted this pass; the rest stay
/// queued — the visible queue.
pub async fn admit_ready(
    board: &dyn BoardStore,
    capacity: usize,
    roster: usize,
) -> PortResult<Vec<orexis_core::BoardTask>> {
    let slots = capacity
        .min(roster)
        .saturating_sub(running_count(board).await?);
    let mut admitted = Vec::new();
    if slots == 0 {
        return Ok(admitted);
    }
    for task in board.ready().await? {
        if admitted.len() >= slots {
            break;
        }
        admitted.push(board.admit(&task.id).await?);
    }
    Ok(admitted)
}

async fn running_count(board: &dyn BoardStore) -> PortResult<usize> {
    Ok(board
        .list()
        .await?
        .iter()
        .filter(|task| task.status == TaskStatus::Running)
        .count())
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::executor::block_on;
    use orexis_core::BoardTask;
    use orexis_ports::PortError;
    use std::collections::BTreeMap;
    use std::sync::Mutex;

    use async_trait::async_trait;

    /// A test-local board fake: use cases depend on ports only, so the
    /// fake lives here, not in an adapter crate.
    struct TestBoard {
        tasks: Mutex<BTreeMap<String, BoardTask>>,
        next: Mutex<u64>,
    }

    impl TestBoard {
        fn new() -> Self {
            Self {
                tasks: Mutex::new(BTreeMap::new()),
                next: Mutex::new(1),
            }
        }

        async fn seed(&self, titles: &[&str]) {
            for title in titles {
                self.create(title, "build", vec![]).await.expect("create");
            }
        }
    }

    #[async_trait]
    impl BoardStore for TestBoard {
        async fn create(
            &self,
            title: &str,
            required_strategy: &str,
            depends_on: Vec<String>,
        ) -> PortResult<BoardTask> {
            let mut tasks = self.tasks.lock().expect("tasks");
            let mut next = self.next.lock().expect("seq");
            let id = format!("task-{}", *next);
            *next += 1;
            let task = BoardTask {
                id: id.clone(),
                title: title.to_string(),
                required_strategy: required_strategy.to_string(),
                owner: None,
                status: orexis_core::TaskStatus::Queued,
                depends_on,
                seq: *next,
            };
            tasks.insert(id, task.clone());
            Ok(task)
        }

        async fn get(&self, id: &str) -> PortResult<Option<BoardTask>> {
            Ok(self.tasks.lock().expect("tasks").get(id).cloned())
        }

        async fn list(&self) -> PortResult<Vec<BoardTask>> {
            Ok(self
                .tasks
                .lock()
                .expect("tasks")
                .values()
                .cloned()
                .collect())
        }

        async fn ready(&self) -> PortResult<Vec<BoardTask>> {
            let tasks = self.tasks.lock().expect("tasks");
            Ok(tasks
                .values()
                .filter(|task| {
                    task.status == orexis_core::TaskStatus::Queued
                        && task.depends_on.iter().all(|edge| {
                            tasks
                                .get(edge)
                                .is_some_and(|dep| dep.status == orexis_core::TaskStatus::Done)
                        })
                })
                .cloned()
                .collect())
        }

        async fn admit(&self, id: &str) -> PortResult<BoardTask> {
            let mut tasks = self.tasks.lock().expect("tasks");
            let task = tasks
                .get_mut(id)
                .ok_or_else(|| PortError(format!("no task {id}")))?;
            task.status = orexis_core::TaskStatus::Admitted;
            Ok(task.clone())
        }

        async fn start(&self, id: &str, owner: &str) -> PortResult<BoardTask> {
            let mut tasks = self.tasks.lock().expect("tasks");
            let task = tasks
                .get_mut(id)
                .ok_or_else(|| PortError(format!("no task {id}")))?;
            task.status = orexis_core::TaskStatus::Running;
            task.owner = Some(owner.to_string());
            Ok(task.clone())
        }

        async fn complete(&self, id: &str) -> PortResult<BoardTask> {
            let mut tasks = self.tasks.lock().expect("tasks");
            let task = tasks
                .get_mut(id)
                .ok_or_else(|| PortError(format!("no task {id}")))?;
            task.status = orexis_core::TaskStatus::Done;
            Ok(task.clone())
        }

        async fn fail(&self, id: &str, reason: &str) -> PortResult<BoardTask> {
            let mut tasks = self.tasks.lock().expect("tasks");
            let task = tasks
                .get_mut(id)
                .ok_or_else(|| PortError(format!("no task {id}")))?;
            task.status = orexis_core::TaskStatus::Failed;
            task.title = format!("{} — failed: {reason}", task.title);
            Ok(task.clone())
        }
    }

    #[test]
    fn admission_respects_the_providers_capacity() {
        let board = TestBoard::new();
        block_on(board.seed(&["a", "b", "c"]));
        // Capacity 2: two admitted, one stays queued — the visible queue.
        let admitted = block_on(admit_ready(&board, 2, 16)).expect("admit");
        assert_eq!(admitted.len(), 2);
        let queued = block_on(board.ready()).expect("ready");
        assert_eq!(queued.len(), 1, "the queue is visible, not dropped");
    }

    #[test]
    fn the_roster_caps_admission() {
        let board = TestBoard::new();
        block_on(board.seed(&["a", "b", "c"]));
        // A huge provider capacity cannot exceed the roster.
        let admitted = block_on(admit_ready(&board, 100, 2)).expect("admit");
        assert_eq!(admitted.len(), 2);
    }

    #[test]
    fn running_tasks_occupy_slots() {
        let board = TestBoard::new();
        block_on(board.seed(&["a", "b"]));
        // One task already running: capacity 2 admits only one more.
        let tasks = block_on(board.list()).expect("list");
        block_on(board.admit(&tasks[0].id)).expect("admit");
        block_on(board.start(&tasks[0].id, "worker-1")).expect("start");
        let admitted = block_on(admit_ready(&board, 2, 16)).expect("admit");
        assert_eq!(admitted.len(), 1);
    }

    #[test]
    fn zero_capacity_admits_nothing() {
        let board = TestBoard::new();
        block_on(board.seed(&["a"]));
        let admitted = block_on(admit_ready(&board, 0, 16)).expect("admit");
        assert!(admitted.is_empty());
        // A local provider's single generation slot is respected.
        let admitted = block_on(admit_ready(&board, 1, 16)).expect("admit");
        assert_eq!(admitted.len(), 1);
    }

    #[test]
    fn edges_gate_admission() {
        let board = TestBoard::new();
        block_on(async {
            let first = board
                .create("first", "build", vec![])
                .await
                .expect("create");
            board
                .create("second", "build", vec![first.id.clone()])
                .await
                .expect("create");
            // Only the edgeless task is ready; the dependent stays queued.
            let admitted = admit_ready(&board, 4, 16).await.expect("admit");
            assert_eq!(admitted.len(), 1);
            assert_eq!(admitted[0].id, first.id);
        });
    }
}
