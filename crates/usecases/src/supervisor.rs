//! The supervisor (M8-08, ADR-0020): a watchdog over workers. It reads
//! the board, finds running tasks whose worker is not live, and plans
//! the response: restart the worker on its own session, or — after N
//! restarts — fail the task and tell the bus. It never runs turns
//! itself; the team loop (M8-09) executes the plan.

use std::collections::BTreeMap;

use orexis_core::TaskStatus;
use orexis_ports::{BoardStore, MessageStore, PortResult};

/// How many restarts a stalled task gets before escalation.
pub const MAX_RESTARTS: u8 = 2;

/// The supervisor's memory: restart counts per task. The team loop holds
/// it across ticks; a task that finishes clears its count.
#[derive(Default)]
pub struct SupervisorState {
    restarts: BTreeMap<String, u8>,
}

impl SupervisorState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Drop the count of a task that reached a terminal state.
    pub fn settled(&mut self, task_id: &str) {
        self.restarts.remove(task_id);
    }
}

/// What the team loop should do this tick.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Supervision {
    /// Running tasks whose worker is gone but still under the restart
    /// budget: re-spawn the worker on the task's session.
    pub restart: Vec<orexis_core::BoardTask>,
    /// Tasks that exhausted the restart budget: already failed on the
    /// board; the bus was told.
    pub escalated: Vec<orexis_core::BoardTask>,
}

/// One supervision tick. `live` carries the task ids with a live worker
/// turn; a running task absent from it is stalled — a dead turn (a
/// panic), a lost daemon, or a worker that never started.
pub async fn supervise(
    state: &mut SupervisorState,
    board: &dyn BoardStore,
    bus: &dyn MessageStore,
    live: &[String],
    max_restarts: u8,
) -> PortResult<Supervision> {
    let mut plan = Supervision::default();
    for task in board.list().await? {
        match task.status {
            TaskStatus::Done | TaskStatus::Failed => {
                state.settled(&task.id);
            }
            TaskStatus::Running if !live.iter().any(|id| id == &task.id) => {
                let count = state.restarts.entry(task.id.clone()).or_insert(0);
                *count += 1;
                if *count > max_restarts {
                    let failed = board
                        .fail(&task.id, &format!("stalled; restarted {count} times"))
                        .await?;
                    bus.post(
                        "supervisor",
                        None,
                        &format!(
                            "task {} escalated: stalled, restarted {count} times, failed",
                            task.id
                        ),
                    )
                    .await?;
                    state.settled(&task.id);
                    plan.escalated.push(failed);
                } else {
                    plan.restart.push(task);
                }
            }
            _ => {}
        }
    }
    Ok(plan)
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use futures::executor::block_on;
    use orexis_core::BoardTask;
    use orexis_core::BusMessage;
    use orexis_ports::PortError;
    use std::sync::Mutex;

    /// Test-local fakes: use cases depend on ports only.
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

        async fn running(&self, title: &str) -> BoardTask {
            let task = self.create(title, "build", vec![]).await.expect("create");
            self.admit(&task.id).await.expect("admit");
            self.start(&task.id, "worker-1").await.expect("start")
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
                status: TaskStatus::Queued,
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
            Ok(self
                .tasks
                .lock()
                .expect("tasks")
                .values()
                .filter(|task| task.status == TaskStatus::Queued)
                .cloned()
                .collect())
        }

        async fn admit(&self, id: &str) -> PortResult<BoardTask> {
            let mut tasks = self.tasks.lock().expect("tasks");
            let task = tasks
                .get_mut(id)
                .ok_or_else(|| PortError(format!("no task {id}")))?;
            task.status = TaskStatus::Admitted;
            Ok(task.clone())
        }

        async fn start(&self, id: &str, owner: &str) -> PortResult<BoardTask> {
            let mut tasks = self.tasks.lock().expect("tasks");
            let task = tasks
                .get_mut(id)
                .ok_or_else(|| PortError(format!("no task {id}")))?;
            task.status = TaskStatus::Running;
            task.owner = Some(owner.to_string());
            Ok(task.clone())
        }

        async fn complete(&self, id: &str) -> PortResult<BoardTask> {
            let mut tasks = self.tasks.lock().expect("tasks");
            let task = tasks
                .get_mut(id)
                .ok_or_else(|| PortError(format!("no task {id}")))?;
            task.status = TaskStatus::Done;
            Ok(task.clone())
        }

        async fn fail(&self, id: &str, reason: &str) -> PortResult<BoardTask> {
            let mut tasks = self.tasks.lock().expect("tasks");
            let task = tasks
                .get_mut(id)
                .ok_or_else(|| PortError(format!("no task {id}")))?;
            task.status = TaskStatus::Failed;
            task.title = format!("{} — failed: {reason}", task.title);
            Ok(task.clone())
        }
    }

    #[derive(Default)]
    struct TestBus {
        messages: Mutex<Vec<BusMessage>>,
    }

    #[async_trait]
    impl MessageStore for TestBus {
        async fn post(&self, from: &str, to: Option<&str>, text: &str) -> PortResult<BusMessage> {
            let mut messages = self.messages.lock().expect("messages");
            let seq = messages.last().map(|message| message.seq + 1).unwrap_or(1);
            let message = BusMessage {
                seq,
                from: from.to_string(),
                to: to.map(str::to_string),
                text: text.to_string(),
            };
            messages.push(message.clone());
            Ok(message)
        }

        async fn read_since(
            &self,
            seq: u64,
            for_worker: Option<&str>,
        ) -> PortResult<Vec<BusMessage>> {
            Ok(self
                .messages
                .lock()
                .expect("messages")
                .iter()
                .filter(|message| {
                    message.seq > seq
                        && match for_worker {
                            None => true,
                            Some(worker) => {
                                message.to.is_none() || message.to.as_deref() == Some(worker)
                            }
                        }
                })
                .cloned()
                .collect())
        }
    }

    #[test]
    fn a_stalled_task_is_restarted_within_the_budget() {
        let board = TestBoard::new();
        let task = block_on(board.running("stalled"));
        let bus = TestBus::default();
        let mut state = SupervisorState::new();

        let plan =
            block_on(supervise(&mut state, &board, &bus, &[], MAX_RESTARTS)).expect("supervise");
        assert_eq!(plan.restart, vec![task.clone()]);
        assert!(plan.escalated.is_empty());
        assert!(
            block_on(board.get(&task.id))
                .expect("get")
                .expect("task")
                .status
                == TaskStatus::Running,
            "a restart does not fail the task"
        );
    }

    #[test]
    fn a_live_task_is_left_alone() {
        let board = TestBoard::new();
        block_on(board.running("live"));
        let bus = TestBus::default();
        let mut state = SupervisorState::new();

        let plan = block_on(supervise(
            &mut state,
            &board,
            &bus,
            &["task-1".to_string()],
            MAX_RESTARTS,
        ))
        .expect("supervise");
        assert!(plan.restart.is_empty() && plan.escalated.is_empty());
    }

    #[test]
    fn exhausting_the_budget_escalates_and_tells_the_bus() {
        let board = TestBoard::new();
        let task = block_on(board.running("stuck"));
        let bus = TestBus::default();
        let mut state = SupervisorState::new();

        // Two restarts planned, then the third tick escalates.
        for _ in 0..MAX_RESTARTS {
            let plan = block_on(supervise(&mut state, &board, &bus, &[], MAX_RESTARTS))
                .expect("supervise");
            assert_eq!(plan.restart.len(), 1, "still under the budget");
        }
        let plan =
            block_on(supervise(&mut state, &board, &bus, &[], MAX_RESTARTS)).expect("supervise");
        assert!(plan.restart.is_empty());
        assert_eq!(plan.escalated.len(), 1);
        let failed = block_on(board.get(&task.id)).expect("get").expect("task");
        assert_eq!(failed.status, TaskStatus::Failed);
        assert!(
            failed.title.contains("restarted"),
            "the reason is on the board"
        );
        let messages = block_on(bus.read_since(0, None)).expect("bus");
        assert_eq!(messages.len(), 1, "the bus was told once");
        assert!(messages[0].text.contains("escalated"));
        // The count is cleared; a later tick finds nothing.
        let plan =
            block_on(supervise(&mut state, &board, &bus, &[], MAX_RESTARTS)).expect("supervise");
        assert!(plan.restart.is_empty() && plan.escalated.is_empty());
    }

    #[test]
    fn settled_tasks_clear_their_counts() {
        let board = TestBoard::new();
        let task = block_on(board.running("recovers"));
        let bus = TestBus::default();
        let mut state = SupervisorState::new();

        // One stall observed, then the worker comes back and finishes.
        let _ = block_on(supervise(&mut state, &board, &bus, &[], MAX_RESTARTS)).expect("tick");
        block_on(board.complete(&task.id)).expect("complete");
        let _ = block_on(supervise(&mut state, &board, &bus, &[], MAX_RESTARTS)).expect("tick");

        // A new stall starts from zero, not from the old count.
        let second = block_on(board.running("stalls again"));
        let _ = block_on(supervise(&mut state, &board, &bus, &[], MAX_RESTARTS)).expect("tick");
        let plan = block_on(supervise(&mut state, &board, &bus, &[], MAX_RESTARTS)).expect("tick");
        assert_eq!(plan.restart, vec![second], "the count restarted from zero");
        assert!(plan.escalated.is_empty());
    }
}
