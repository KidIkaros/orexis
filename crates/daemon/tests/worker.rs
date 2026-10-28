//! Integration: a worker's turn (M8-07, ADR-0020) — the existing loop
//! over its own session, shaped by its strategy: identity shared on the
//! first turn, the toolset narrowed, the budget enforced.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use orexis_adapters_codec::NativeCodec;
use orexis_adapters_entity_sqlite::SqliteEntityFactory;
use orexis_adapters_fake::SlowProvider;
use orexis_adapters_session_sqlite::SqliteSessionFactory;
use orexis_adapters_tools::LocalTools;
use orexis_core::{Action, EntityEvent, Message, PermissionPolicy, ProviderResponse, ToolSpec};
use orexis_daemon::{
    spawn_worker_turn, ApprovalHub, DaemonConfig, MemoryOverrides, SessionRegistry, SessionRuntime,
    WorkerTurn,
};
use orexis_ports::{PortResult, Provider};
use orexis_usecases::{strategy, StrategyId};
use tokio::sync::Semaphore;

fn config(dir: &std::path::Path, provider: Arc<dyn Provider>) -> DaemonConfig {
    DaemonConfig {
        socket: dir.join("unused.sock"),
        profile_id: "fake".into(),
        model_id: "fake".into(),
        sessions: Arc::new(SqliteSessionFactory::new(dir.join("sessions.db"))),
        entity: Arc::new(SqliteEntityFactory::new(dir.join("entity.db"))),
        identity_path: None,
        evolution: None,
        board: Arc::new(orexis_adapters_fake::FakeBoard::new(256)),
        bus: Arc::new(orexis_adapters_fake::FakeBus::new()),
        blackboard: Arc::new(orexis_adapters_fake::FakeBlackboard::new()),
        team: false,
        external: Vec::new(),
        provider,
        tools: Arc::new(LocalTools::new(dir).expect("tools")),
        workspace: Arc::new(LocalTools::new(dir).expect("workspace")),
        codec: Arc::new(NativeCodec),
        policy: Arc::new(PermissionPolicy::standard()),
        checkpointer: None,
        verifier: None,
        system_prompt: String::new(),
        context_tools: Vec::new(),
        context_files: Vec::new(),
        context_limit: 32768,
        max_steps: 8,
        generation_slots: 1,
    }
}

/// A provider that records the messages it is handed.
struct RecordingProvider {
    script: Vec<Vec<Action>>,
    calls: Arc<Mutex<Vec<Vec<Message>>>>,
}

impl RecordingProvider {
    fn new(script: Vec<Vec<Action>>) -> (Self, Arc<Mutex<Vec<Vec<Message>>>>) {
        let calls = Arc::new(Mutex::new(Vec::new()));
        (
            Self {
                script,
                calls: calls.clone(),
            },
            calls,
        )
    }
}

#[async_trait]
impl Provider for RecordingProvider {
    fn capacity(&self) -> usize {
        1
    }

    async fn complete(
        &self,
        messages: &[Message],
        _tools: &[ToolSpec],
        step: usize,
        _thinking: orexis_core::Thinking,
    ) -> PortResult<ProviderResponse> {
        self.calls.lock().expect("calls").push(messages.to_vec());
        let actions = self.script.get(step).cloned().unwrap_or_else(|| {
            vec![Action::Done {
                summary: "script exhausted".into(),
            }]
        });
        Ok(ProviderResponse {
            actions,
            raw_text: String::new(),
            usage: None,
        })
    }
}

async fn worker_outcome(
    dir: &std::path::Path,
    provider: Arc<dyn Provider>,
    prompt: &str,
    worker: WorkerTurn,
) -> orexis_daemon::WorkerOutcome {
    let config = Arc::new(config(dir, provider));
    let registry = SessionRegistry::new(config.sessions.clone());
    let rt: Arc<SessionRuntime> = registry.get_or_create(None).expect("session");
    let hub = Arc::new(ApprovalHub::new(Arc::new(MemoryOverrides::default())));
    let permit = Arc::new(Semaphore::new(1))
        .acquire_owned()
        .await
        .expect("permit");
    let handle = spawn_worker_turn(
        &rt,
        &config,
        &config.tools,
        &hub,
        prompt.to_string(),
        worker,
        permit,
    )
    .await
    .expect("spawn worker");
    handle.await.expect("worker task")
}

#[tokio::test]
async fn a_worker_runs_to_done_on_its_own_session() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (provider, _calls) = RecordingProvider::new(vec![vec![Action::Done {
        summary: "the task is done".into(),
    }]]);
    let build = strategy(StrategyId::Build);
    let outcome = worker_outcome(
        dir.path(),
        Arc::new(provider),
        "do the thing",
        WorkerTurn {
            prompt_suffix: format!(
                "You are working on board task task-1: do the thing.\n{}",
                build.prompt_fragment
            ),
            toolset: build.toolset.clone(),
            max_steps: build.budget.max_steps,
            wall_clock: Duration::from_secs(60),
        },
    )
    .await;
    assert!(outcome.done, "outcome: {outcome:?}");
    assert!(outcome.error.is_none());
}

#[tokio::test]
async fn the_entity_identity_is_shared() {
    let dir = tempfile::tempdir().expect("tempdir");
    // The entity holds a trait; the worker's first turn must see it.
    let entity = SqliteEntityFactory::new(dir.path().join("entity.db"));
    {
        let refs = orexis_ports::EntityFactory::open(&entity).expect("entity");
        refs.append
            .append(EntityEvent::TraitUpdate {
                trait_id: "style".into(),
                statement: "the team prefers small diffs".into(),
                confidence: 0.9,
                evidence: vec![1],
                last_seen_at: 0,
            })
            .await
            .expect("trait");
    }
    let (provider, calls) = RecordingProvider::new(vec![vec![Action::Done {
        summary: "done".into(),
    }]]);
    let provider: Arc<dyn Provider> = Arc::new(provider);
    let config = Arc::new(config(dir.path(), provider));
    let registry = SessionRegistry::new(config.sessions.clone());
    let rt = registry.get_or_create(None).expect("session");
    let hub = Arc::new(ApprovalHub::new(Arc::new(MemoryOverrides::default())));
    let permit = Arc::new(Semaphore::new(1))
        .acquire_owned()
        .await
        .expect("permit");
    let build = strategy(StrategyId::Build);
    let handle = spawn_worker_turn(
        &rt,
        &config,
        &config.tools,
        &hub,
        "do the thing".to_string(),
        WorkerTurn {
            prompt_suffix: format!(
                "You are working on board task task-1: do the thing.\n{}",
                build.prompt_fragment
            ),
            toolset: build.toolset.clone(),
            max_steps: build.budget.max_steps,
            wall_clock: Duration::from_secs(60),
        },
        permit,
    )
    .await
    .expect("spawn worker");
    let outcome = handle.await.expect("worker task");
    assert!(outcome.done, "outcome: {outcome:?}");

    let calls = calls.lock().expect("calls");
    let system = &calls[0]
        .iter()
        .find(|message| matches!(message.role, orexis_core::Role::System));
    assert!(
        system
            .as_ref()
            .is_some_and(|message| message.text.contains("the team prefers small diffs")),
        "the worker's first turn carries the entity's trait"
    );
}

#[tokio::test]
async fn the_strategy_toolset_narrows_the_gate() {
    let dir = tempfile::tempdir().expect("tempdir");
    // The plan strategy may not edit files; the worker calls write_file
    // and the turn fails visibly.
    let (provider, _calls) = RecordingProvider::new(vec![vec![Action::CallTool {
        call_id: "call-1".into(),
        name: "write_file".into(),
        args: r#"{"path":"x.txt","content":"no"}"#.into(),
    }]]);
    let plan = strategy(StrategyId::Plan);
    let outcome = worker_outcome(
        dir.path(),
        Arc::new(provider),
        "plan the work",
        WorkerTurn {
            prompt_suffix: plan.prompt_fragment.to_string(),
            toolset: plan.toolset.clone(),
            max_steps: plan.budget.max_steps,
            wall_clock: Duration::from_secs(60),
        },
    )
    .await;
    assert!(!outcome.done, "outcome: {outcome:?}");
    assert!(
        outcome
            .error
            .as_deref()
            .is_some_and(|error| error.contains("outside this strategy's toolset")),
        "the narrowing is the error: {outcome:?}"
    );
}

#[tokio::test]
async fn the_step_budget_exhausts_visibly() {
    let dir = tempfile::tempdir().expect("tempdir");
    // A provider that never finishes: the turn ends at max_steps without
    // a Done, and the outcome says so.
    let (provider, _calls) = RecordingProvider::new(vec![
        vec![Action::Say {
            text: "still working".into(),
        }];
        8
    ]);
    let build = strategy(StrategyId::Build);
    let outcome = worker_outcome(
        dir.path(),
        Arc::new(provider),
        "never finishes",
        WorkerTurn {
            prompt_suffix: build.prompt_fragment.to_string(),
            toolset: build.toolset.clone(),
            max_steps: 3,
            wall_clock: Duration::from_secs(60),
        },
    )
    .await;
    assert!(!outcome.done, "outcome: {outcome:?}");
    assert_eq!(outcome.steps, 3, "the turn stopped at the budget");
    assert!(outcome.error.is_none(), "exhaustion is not an error");
}

#[tokio::test]
async fn the_wall_clock_exhausts_visibly() {
    let dir = tempfile::tempdir().expect("tempdir");
    // A slow provider against a tiny wall clock: the timeout is the
    // outcome.
    let provider: Arc<dyn Provider> = Arc::new(SlowProvider::new(Duration::from_secs(30)));
    let build = strategy(StrategyId::Build);
    let outcome = worker_outcome(
        dir.path(),
        provider,
        "too slow",
        WorkerTurn {
            prompt_suffix: build.prompt_fragment.to_string(),
            toolset: build.toolset.clone(),
            max_steps: build.budget.max_steps,
            wall_clock: Duration::from_millis(300),
        },
    )
    .await;
    assert!(!outcome.done, "outcome: {outcome:?}");
    assert!(
        outcome
            .error
            .as_deref()
            .is_some_and(|error| error.contains("wall clock")),
        "the timeout is the error: {outcome:?}"
    );
}
