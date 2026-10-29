//! The team's board tools (M8-11, ADR-0020): the model's write surface
//! for the board — `board_add` creates a task, `board_list` reads it.
//! They are team-domain tools, not workspace tools: `board_add` mutates
//! the board, never the repository, so plan mode allows it and the plan
//! strategy's toolset includes it. Other names fall through to the
//! wrapped executor untouched.

use std::sync::Arc;

use async_trait::async_trait;
use orexis_core::{render_task, Observation, ToolSpec};
use orexis_ports::{BoardStore, PortError, PortResult, ToolExecutor};
use serde_json::Value;

pub struct TeamTools {
    board: Arc<dyn BoardStore>,
    inner: Arc<dyn ToolExecutor>,
    /// The strategy names a task may require; the daemon knows the set.
    strategies: Vec<String>,
}

impl TeamTools {
    pub fn new(
        board: Arc<dyn BoardStore>,
        inner: Arc<dyn ToolExecutor>,
        strategies: Vec<String>,
    ) -> Self {
        Self {
            board,
            inner,
            strategies,
        }
    }

    /// The board tools' specs, for the model's tool list.
    pub fn specs() -> Vec<ToolSpec> {
        vec![
            ToolSpec {
                name: "board_add".to_string(),
                description: "Add a task to the team's board".to_string(),
                schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "title": {"type": "string"},
                        "strategy": {"type": "string",
                            "description": "plan, build, review, or mine"},
                        "depends_on": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "task ids that must finish first"
                        }
                    },
                    "required": ["title", "strategy"]
                }),
            },
            ToolSpec {
                name: "board_list".to_string(),
                description: "List the team's board tasks".to_string(),
                schema: serde_json::json!({"type": "object", "properties": {}}),
            },
        ]
    }
}

/// Board output is delimited like any tool output (invariant 22); a
/// closing tag inside it must not end the region early.
fn delimit(name: &str, output: &str) -> String {
    let escaped = output.replace("</tool_result", "<\\/tool_result");
    format!("<tool_result name=\"{name}\">\n{escaped}\n</tool_result>")
}

#[async_trait]
impl ToolExecutor for TeamTools {
    async fn execute(&self, name: &str, args: &str) -> PortResult<Observation> {
        match name {
            "board_add" => {
                let value: Value = serde_json::from_str(args)
                    .map_err(|error| PortError(format!("invalid args: {error}")))?;
                let title = value
                    .get("title")
                    .and_then(Value::as_str)
                    .ok_or_else(|| PortError("board_add needs a title".into()))?;
                let strategy = value
                    .get("strategy")
                    .and_then(Value::as_str)
                    .ok_or_else(|| PortError("board_add needs a strategy".into()))?;
                if !self.strategies.iter().any(|known| known == strategy) {
                    return Err(PortError(format!(
                        "board_add: unknown strategy {strategy} (one of: {})",
                        self.strategies.join(", ")
                    )));
                }
                let depends_on = value
                    .get("depends_on")
                    .and_then(Value::as_array)
                    .map(|edges| {
                        edges
                            .iter()
                            .filter_map(|edge| edge.as_str().map(str::to_string))
                            .collect()
                    })
                    .unwrap_or_default();
                // The board is capped (M8 limits): a runaway planner gets
                // an error, not an unbounded task list.
                let existing = self.board.list().await?;
                // A small planner re-adds the same task every few steps (M8
                // smoke run: alpha.txt three times); the board names the
                // existing one instead of growing a copy.
                let wanted = title.trim().to_lowercase();
                if let Some(same) = existing
                    .iter()
                    .find(|task| task.title.trim().to_lowercase() == wanted)
                {
                    // An observation, not an error: a tool error ends a
                    // worker's turn and fails its task, and a worker
                    // re-adding the task it is already doing (smoke run 3)
                    // is a harmless mistake the model can read and move on from.
                    let output = format!(
                        "not added: \"{}\" is already on the board as {} ({:?})",
                        same.title, same.id, same.status
                    );
                    return Ok(Observation {
                        name: name.to_string(),
                        output: delimit(name, &output),
                    });
                }
                if existing.len() >= orexis_core::BOARD_CAP {
                    return Err(PortError(format!(
                        "the board is full ({} tasks)",
                        orexis_core::BOARD_CAP
                    )));
                }
                let task = self.board.create(title, strategy, depends_on).await?;
                Ok(Observation {
                    name: name.to_string(),
                    output: delimit(name, &render_task(&task)),
                })
            }
            "board_list" => {
                let tasks = self.board.list().await?;
                let rendered = if tasks.is_empty() {
                    "the board is empty".to_string()
                } else {
                    tasks.iter().map(render_task).collect::<Vec<_>>().join("\n")
                };
                Ok(Observation {
                    name: name.to_string(),
                    output: delimit(name, &rendered),
                })
            }
            _ => self.inner.execute(name, args).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use orexis_core::{BoardTask, TaskStatus};
    use std::collections::BTreeMap;
    use std::sync::Mutex;

    /// A test-local board fake: adapters depend on core + ports only.
    #[derive(Default)]
    struct TestBoard {
        tasks: Mutex<BTreeMap<String, BoardTask>>,
        next: Mutex<u64>,
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
                .cloned()
                .collect())
        }

        async fn admit(&self, id: &str) -> PortResult<BoardTask> {
            let mut tasks = self.tasks.lock().expect("tasks");
            let task = tasks
                .get_mut(id)
                .ok_or_else(|| PortError(format!("no task {id}")))?;
            Ok(task.clone())
        }

        async fn start(&self, id: &str, owner: &str) -> PortResult<BoardTask> {
            let mut tasks = self.tasks.lock().expect("tasks");
            let task = tasks
                .get_mut(id)
                .ok_or_else(|| PortError(format!("no task {id}")))?;
            task.owner = Some(owner.to_string());
            Ok(task.clone())
        }

        async fn complete(&self, id: &str) -> PortResult<BoardTask> {
            let tasks = self.tasks.lock().expect("tasks");
            Ok(tasks.get(id).expect("task").clone())
        }

        async fn fail(&self, id: &str, _reason: &str) -> PortResult<BoardTask> {
            let tasks = self.tasks.lock().expect("tasks");
            Ok(tasks.get(id).expect("task").clone())
        }
    }

    struct EchoExecutor;

    #[async_trait]
    impl ToolExecutor for EchoExecutor {
        async fn execute(&self, name: &str, _args: &str) -> PortResult<Observation> {
            Ok(Observation {
                name: name.to_string(),
                output: "ran".to_string(),
            })
        }
    }

    fn tools() -> (Arc<TestBoard>, TeamTools) {
        let board = Arc::new(TestBoard::default());
        let tools = TeamTools::new(
            board.clone(),
            Arc::new(EchoExecutor),
            vec!["plan".to_string(), "build".to_string()],
        );
        (board, tools)
    }

    #[tokio::test]
    async fn board_add_creates_and_renders_delimited() {
        let (_board, tools) = tools();
        let observation = tools
            .execute("board_add", r#"{"title":"fix the bug","strategy":"build"}"#)
            .await
            .expect("add");
        assert!(
            observation
                .output
                .starts_with("<tool_result name=\"board_add\">"),
            "output: {}",
            observation.output
        );
        assert!(observation.output.contains("fix the bug"));
        assert!(observation.output.contains("[queued] build"));
        assert!(observation.output.trim_end().ends_with("</tool_result>"));
    }

    #[tokio::test]
    async fn board_list_renders_the_board() {
        let (board, tools) = tools();
        block_on(board.create("a task", "plan", vec![])).expect("create");
        let observation = tools.execute("board_list", "{}").await.expect("list");
        assert!(observation.output.contains("a task"));
        assert!(observation.output.contains("[queued] plan"));
    }

    #[tokio::test]
    async fn bad_args_and_unknown_strategies_are_refused() {
        let (_board, tools) = tools();
        assert!(tools.execute("board_add", "{}").await.is_err());
        assert!(tools
            .execute("board_add", r#"{"title":"t"}"#)
            .await
            .is_err());
        assert!(tools.execute("board_add", "not json").await.is_err());
        // A strategy the team does not ship never reaches the board.
        let error = tools
            .execute("board_add", r#"{"title":"t","strategy":"execute"}"#)
            .await
            .expect_err("unknown strategy");
        assert!(error.to_string().contains("unknown strategy"));
    }

    #[tokio::test]
    async fn board_add_refuses_a_duplicate_title() {
        let (board, tools) = tools();
        block_on(board.create("Fix alpha.txt", "build", vec![])).expect("create");
        let observation = tools
            .execute(
                "board_add",
                r#"{"title":"  fix ALPHA.txt ","strategy":"build"}"#,
            )
            .await
            .expect("a duplicate is an observation, not a failed turn");
        assert!(observation.output.contains("not added"));
        assert!(observation.output.contains("already on the board"));
        assert_eq!(block_on(board.list()).expect("list").len(), 1);
    }

    #[tokio::test]
    async fn board_add_is_capped() {
        let (board, tools) = tools();
        for index in 0..orexis_core::BOARD_CAP {
            block_on(board.create(&format!("task {index}"), "build", vec![])).expect("create");
        }
        let error = tools
            .execute(
                "board_add",
                r#"{"title":"one too many","strategy":"build"}"#,
            )
            .await
            .expect_err("full board");
        assert!(error.to_string().contains("full"));
    }

    #[tokio::test]
    async fn other_names_fall_through() {
        let (_board, tools) = tools();
        let observation = tools
            .execute("read_file", "{}")
            .await
            .expect("fall through");
        assert_eq!(observation.output, "ran");
    }

    fn block_on<T>(future: impl std::future::Future<Output = T>) -> T {
        futures::executor::block_on(future)
    }
}
