//! Integration: the team loop (M8-09, ADR-0020) — opt-in; when on,
//! board tasks flow to done through workers; when off, nothing runs and
//! the single-agent behavior is unchanged.

use std::sync::Arc;
use std::time::Duration;

use orexis_adapters_codec::NativeCodec;
use orexis_adapters_entity_sqlite::SqliteEntityFactory;
use orexis_adapters_fake::ScriptedProvider;
use orexis_adapters_session_sqlite::SqliteSessionFactory;
use orexis_adapters_tools::LocalTools;
use orexis_core::{Action, PermissionPolicy, TaskStatus};
use orexis_daemon::{Daemon, DaemonConfig};

fn config(dir: &std::path::Path, team: bool) -> DaemonConfig {
    DaemonConfig {
        socket: dir.join("team.sock"),
        profile_id: "fake".into(),
        model_id: "fake".into(),
        sessions: Arc::new(SqliteSessionFactory::new(dir.join("sessions.db"))),
        entity: Arc::new(SqliteEntityFactory::new(dir.join("entity.db"))),
        identity_path: None,
        evolution: None,
        board: Arc::new(orexis_adapters_fake::FakeBoard::new(256)),
        bus: Arc::new(orexis_adapters_fake::FakeBus::new()),
        blackboard: Arc::new(orexis_adapters_fake::FakeBlackboard::new()),
        team,
        external: Vec::new(),
        provider: Arc::new(ScriptedProvider::new(vec![vec![Action::Done {
            summary: "the task is done".into(),
        }]])),
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

/// Wait until the predicate holds, or panic after the timeout.
async fn wait_for(what: &str, seconds: u64, mut predicate: impl FnMut() -> bool) {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(seconds);
    while tokio::time::Instant::now() < deadline {
        if predicate() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("timed out waiting for {what}");
}

#[tokio::test]
async fn team_mode_runs_board_tasks_to_done() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config = Arc::new(config(dir.path(), true));
    let daemon = Daemon::new((*config).clone());
    let server = tokio::spawn(daemon.run());

    // The loop ticks every 250ms; two independent tasks flow to done.
    let board = config.board.clone();
    let first = board
        .create("first task", "build", vec![])
        .await
        .expect("create");
    let second = board
        .create("second task", "build", vec![])
        .await
        .expect("create");

    let board_for_wait = board.clone();
    wait_for("both tasks done", 30, move || {
        let done = |id: &str| {
            futures::executor::block_on(async {
                board_for_wait
                    .get(id)
                    .await
                    .expect("get")
                    .is_some_and(|task| task.status == TaskStatus::Done)
            })
        };
        done(&first.id) && done(&second.id)
    })
    .await;

    // The bus was told about the completions.
    let messages = config.bus.read_since(0, None).await.expect("bus");
    assert!(
        messages
            .iter()
            .any(|message| message.text.contains("completed")),
        "the bus carries the completions: {messages:?}"
    );

    // The entity saw the outcomes (invariant 18: one entity, many
    // workers).
    let entity = SqliteEntityFactory::new(dir.path().join("entity.db"));
    let refs = orexis_ports::EntityFactory::open(&entity).expect("entity");
    let events = refs.read.events().await.expect("events");
    assert!(
        !events.is_empty(),
        "the entity recorded the workers' outcomes"
    );

    server.abort();
}

#[tokio::test]
async fn team_off_leaves_the_board_alone() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config = Arc::new(config(dir.path(), false));
    let daemon = Daemon::new((*config).clone());
    let server = tokio::spawn(daemon.run());

    let board = config.board.clone();
    let task = board
        .create("nobody runs this", "build", vec![])
        .await
        .expect("create");

    tokio::time::sleep(Duration::from_secs(2)).await;
    let after = board.get(&task.id).await.expect("get").expect("task");
    assert_eq!(
        after.status,
        TaskStatus::Queued,
        "team mode off: the board is untouched"
    );

    server.abort();
}
