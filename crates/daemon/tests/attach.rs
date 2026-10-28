//! Integration: attach, prompt, detach, reattach, replay, and two sessions.

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use orexis_adapters_codec::NativeCodec;
use orexis_adapters_entity_sqlite::{SqliteEntityFactory, SqliteEntityStore};
use orexis_adapters_fake::{EchoTools, ScriptedProvider};
use orexis_adapters_session_sqlite::SqliteSessionFactory;
use orexis_adapters_tools::LocalTools;
use orexis_core::{
    Action, EntityEvent, Message, Observation, PermissionPolicy, ProviderResponse, Role,
    SessionEvent, ToolSpec,
};
use orexis_daemon::{
    Artifact, ClientMsg, Daemon, DaemonConfig, FileEntry, ServerMsg, SettingsInfo,
};
use orexis_ports::{Checkpointer, EntityAppend, ExternalTools, PortResult, Provider, ToolExecutor};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::unix::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::UnixStream;
use tokio::time::timeout;

fn config(socket: &Path, db: &Path) -> DaemonConfig {
    config_with(
        socket,
        db,
        vec![
            vec![Action::CallTool {
                call_id: "c1".to_string(),
                name: "read_file".to_string(),
                args: "x".to_string(),
            }],
            vec![Action::Done {
                summary: "done".to_string(),
            }],
        ],
    )
}

fn config_with(socket: &Path, db: &Path, script: Vec<Vec<Action>>) -> DaemonConfig {
    DaemonConfig {
        socket: socket.to_path_buf(),
        profile_id: "test".to_string(),
        model_id: "test".to_string(),
        sessions: Arc::new(SqliteSessionFactory::new(db.to_path_buf())),
        entity: Arc::new(SqliteEntityFactory::new(
            socket.parent().unwrap_or(Path::new(".")).join("entity.db"),
        )),
        identity_path: None,
        evolution: None,
        board: Arc::new(orexis_adapters_fake::FakeBoard::new(256)),
        bus: Arc::new(orexis_adapters_fake::FakeBus::new()),
        blackboard: Arc::new(orexis_adapters_fake::FakeBlackboard::new()),
        team: false,
        external: Vec::new(),
        provider: Arc::new(ScriptedProvider::new(script)),
        tools: Arc::new(EchoTools),
        workspace: Arc::new(
            LocalTools::new(socket.parent().unwrap_or(Path::new("."))).expect("workspace"),
        ),
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

async fn wait_for(path: &Path) {
    for _ in 0..200 {
        if path.exists() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("socket was not created");
}

async fn send(writer: &mut OwnedWriteHalf, message: &ClientMsg) {
    let mut line = serde_json::to_string(message).expect("client msg");
    line.push('\n');
    writer.write_all(line.as_bytes()).await.expect("write");
    writer.flush().await.expect("flush");
}

async fn read_server(reader: &mut BufReader<OwnedReadHalf>, line: &mut String) -> ServerMsg {
    line.clear();
    let read = timeout(Duration::from_secs(5), reader.read_line(line))
        .await
        .expect("timeout")
        .expect("read");
    assert!(read > 0, "eof");
    serde_json::from_str(line.trim()).expect("server msg")
}

async fn read_until_done(
    reader: &mut BufReader<OwnedReadHalf>,
    line: &mut String,
) -> Vec<ServerMsg> {
    let mut messages = Vec::new();
    loop {
        let message = read_server(reader, line).await;
        let done = matches!(
            &message,
            ServerMsg::Event {
                event: SessionEvent::Done { .. },
                ..
            }
        );
        messages.push(message);
        if done {
            return messages;
        }
    }
}

#[derive(Default)]
struct RecordingCheckpointer {
    created: Mutex<Vec<String>>,
    restored: Mutex<Vec<String>>,
}

impl Checkpointer for RecordingCheckpointer {
    fn create(&self, _label: &str) -> PortResult<String> {
        let mut created = self.created.lock().expect("created");
        let id = format!("cp-{}", created.len() + 1);
        created.push(id.clone());
        Ok(id)
    }

    fn restore(&self, id: &str) -> PortResult<()> {
        self.restored.lock().expect("restored").push(id.to_string());
        Ok(())
    }

    fn diff(&self, _id: &str) -> PortResult<String> {
        Ok("--- a/a.txt\n+++ b/a.txt\n-one\n+two\n".to_string())
    }
}

async fn spawn_and_attach(
    config: DaemonConfig,
    socket: &Path,
) -> (
    tokio::task::JoinHandle<()>,
    BufReader<OwnedReadHalf>,
    OwnedWriteHalf,
    String,
) {
    let daemon = Daemon::new(config);
    let handle = tokio::spawn(async move {
        let _ = daemon.run().await;
    });
    wait_for(socket).await;
    let (mut reader, mut writer) = connect(socket).await;
    let mut line = String::new();
    send(&mut writer, &ClientMsg::Attach { session: None }).await;
    let _ = read_server(&mut reader, &mut line).await;
    (handle, reader, writer, line)
}

async fn read_until_evolution(
    reader: &mut BufReader<OwnedReadHalf>,
    line: &mut String,
) -> Vec<String> {
    loop {
        match read_server(reader, line).await {
            ServerMsg::Evolution { entries } => return entries,
            ServerMsg::Error { message } => panic!("evolution error: {message}"),
            _ => {}
        }
    }
}

async fn read_until_self_view(reader: &mut BufReader<OwnedReadHalf>, line: &mut String) -> String {
    loop {
        match read_server(reader, line).await {
            ServerMsg::SelfView { text } => return text,
            ServerMsg::Error { message } => panic!("self error: {message}"),
            _ => {}
        }
    }
}

async fn read_until_audit(reader: &mut BufReader<OwnedReadHalf>, line: &mut String) -> Vec<String> {
    loop {
        match read_server(reader, line).await {
            ServerMsg::Audit { entries } => return entries,
            ServerMsg::Error { message } => panic!("audit error: {message}"),
            _ => {}
        }
    }
}

async fn read_until_settings(
    reader: &mut BufReader<OwnedReadHalf>,
    line: &mut String,
) -> SettingsInfo {
    loop {
        match read_server(reader, line).await {
            ServerMsg::Settings { settings } => return settings,
            ServerMsg::Error { message } => panic!("settings error: {message}"),
            _ => {}
        }
    }
}

async fn read_until_dir(
    reader: &mut BufReader<OwnedReadHalf>,
    line: &mut String,
) -> (String, Vec<FileEntry>) {
    loop {
        match read_server(reader, line).await {
            ServerMsg::Dir { path, entries } => return (path, entries),
            ServerMsg::Error { message } => panic!("dir error: {message}"),
            _ => {}
        }
    }
}

async fn read_until_file(
    reader: &mut BufReader<OwnedReadHalf>,
    line: &mut String,
) -> (String, String, bool) {
    loop {
        match read_server(reader, line).await {
            ServerMsg::File {
                path,
                content,
                truncated,
            } => return (path, content, truncated),
            ServerMsg::Error { message } => panic!("file error: {message}"),
            _ => {}
        }
    }
}

async fn read_until_artifacts(
    reader: &mut BufReader<OwnedReadHalf>,
    line: &mut String,
) -> Vec<Artifact> {
    loop {
        match read_server(reader, line).await {
            ServerMsg::Artifacts { items } => return items,
            ServerMsg::Error { message } => panic!("artifacts error: {message}"),
            _ => {}
        }
    }
}

async fn read_until_diff(reader: &mut BufReader<OwnedReadHalf>, line: &mut String) -> String {
    loop {
        match read_server(reader, line).await {
            ServerMsg::Diff { diff } => return diff,
            ServerMsg::Error { message } => panic!("diff error: {message}"),
            _ => {}
        }
    }
}

async fn read_until_info(reader: &mut BufReader<OwnedReadHalf>, line: &mut String) -> String {
    loop {
        let message = read_server(reader, line).await;
        if let ServerMsg::Event {
            event: SessionEvent::Info { message },
            ..
        } = message
        {
            return message;
        }
    }
}

async fn read_until_approval(reader: &mut BufReader<OwnedReadHalf>, line: &mut String) -> u64 {
    loop {
        let message = read_server(reader, line).await;
        if let ServerMsg::ApprovalRequest { id, .. } = message {
            return id;
        }
    }
}

async fn connect(path: &Path) -> (BufReader<OwnedReadHalf>, OwnedWriteHalf) {
    let stream = UnixStream::connect(path).await.expect("connect");
    let (read_half, write_half) = stream.into_split();
    (BufReader::new(read_half), write_half)
}

#[tokio::test]
async fn attach_prompt_detach_reattach_replays() {
    let dir = tempfile::tempdir().expect("tempdir");
    let socket = dir.path().join("daemon.sock");
    let db = dir.path().join("sessions.db");
    let daemon = Daemon::new(config(&socket, &db));
    tokio::spawn(async move {
        let _ = daemon.run().await;
    });
    wait_for(&socket).await;

    let (mut reader, mut writer) = connect(&socket).await;
    let mut line = String::new();

    send(&mut writer, &ClientMsg::Attach { session: None }).await;
    let ServerMsg::Attached { session_id } = read_server(&mut reader, &mut line).await else {
        panic!("expected attached");
    };

    send(
        &mut writer,
        &ClientMsg::Prompt {
            text: "go".to_string(),
            skip_verify: false,
        },
    )
    .await;
    let live = read_until_done(&mut reader, &mut line).await;
    assert!(live.iter().any(|message| matches!(
        message,
        ServerMsg::Event {
            event: SessionEvent::ToolCall { .. },
            ..
        }
    )));
    assert!(live
        .iter()
        .all(|message| !matches!(message, ServerMsg::Event { replay: true, .. })));

    send(&mut writer, &ClientMsg::Detach).await;
    drop(writer);

    let (mut reader, mut writer) = connect(&socket).await;
    let mut line = String::new();
    send(
        &mut writer,
        &ClientMsg::Attach {
            session: Some(session_id.clone()),
        },
    )
    .await;
    let ServerMsg::Attached { session_id: again } = read_server(&mut reader, &mut line).await
    else {
        panic!("expected attached");
    };
    assert_eq!(again, session_id);
    let replay = read_until_done(&mut reader, &mut line).await;
    assert!(replay
        .iter()
        .all(|message| matches!(message, ServerMsg::Event { replay: true, .. })));
    assert!(replay.iter().any(|message| matches!(
        message,
        ServerMsg::Event {
            event: SessionEvent::Done { .. },
            ..
        }
    )));
}

#[tokio::test]
async fn two_sessions_complete_under_one_generation_slot() {
    let dir = tempfile::tempdir().expect("tempdir");
    let socket = dir.path().join("daemon.sock");
    let db = dir.path().join("sessions.db");
    let daemon = Daemon::new(config(&socket, &db));
    tokio::spawn(async move {
        let _ = daemon.run().await;
    });
    wait_for(&socket).await;

    let (mut reader_a, mut writer_a) = connect(&socket).await;
    let (mut reader_b, mut writer_b) = connect(&socket).await;
    let mut line_a = String::new();
    let mut line_b = String::new();

    send(&mut writer_a, &ClientMsg::Attach { session: None }).await;
    let _ = read_server(&mut reader_a, &mut line_a).await;
    send(&mut writer_b, &ClientMsg::Attach { session: None }).await;
    let _ = read_server(&mut reader_b, &mut line_b).await;

    send(
        &mut writer_a,
        &ClientMsg::Prompt {
            text: "a".to_string(),
            skip_verify: false,
        },
    )
    .await;
    send(
        &mut writer_b,
        &ClientMsg::Prompt {
            text: "b".to_string(),
            skip_verify: false,
        },
    )
    .await;

    let (a, b) = tokio::join!(
        read_until_done(&mut reader_a, &mut line_a),
        read_until_done(&mut reader_b, &mut line_b)
    );
    assert!(a.iter().any(|m| matches!(
        m,
        ServerMsg::Event {
            event: SessionEvent::Done { .. },
            ..
        }
    )));
    assert!(b.iter().any(|m| matches!(
        m,
        ServerMsg::Event {
            event: SessionEvent::Done { .. },
            ..
        }
    )));
}

#[tokio::test]
async fn a_second_client_attaches_to_the_same_session() {
    let dir = tempfile::tempdir().expect("tempdir");
    let socket = dir.path().join("daemon.sock");
    let db = dir.path().join("sessions.db");
    let daemon = Daemon::new(config(&socket, &db));
    tokio::spawn(async move {
        let _ = daemon.run().await;
    });
    wait_for(&socket).await;

    let (mut reader_a, mut writer_a) = connect(&socket).await;
    let (mut reader_b, mut writer_b) = connect(&socket).await;
    let mut line_a = String::new();
    let mut line_b = String::new();

    send(&mut writer_a, &ClientMsg::Attach { session: None }).await;
    let ServerMsg::Attached { session_id } = read_server(&mut reader_a, &mut line_a).await else {
        panic!("expected attached");
    };
    send(
        &mut writer_b,
        &ClientMsg::Attach {
            session: Some(session_id),
        },
    )
    .await;
    let _ = read_server(&mut reader_b, &mut line_b).await;

    send(
        &mut writer_a,
        &ClientMsg::Prompt {
            text: "shared".to_string(),
            skip_verify: false,
        },
    )
    .await;
    let (a, b) = tokio::join!(
        read_until_done(&mut reader_a, &mut line_a),
        read_until_done(&mut reader_b, &mut line_b)
    );
    assert!(a.iter().any(|m| matches!(
        m,
        ServerMsg::Event {
            event: SessionEvent::Done { .. },
            ..
        }
    )));
    assert!(b.iter().any(|m| matches!(
        m,
        ServerMsg::Event {
            event: SessionEvent::Done { .. },
            ..
        }
    )));
}

#[tokio::test]
async fn approval_required_tool_blocks_until_approved() {
    let dir = tempfile::tempdir().expect("tempdir");
    let socket = dir.path().join("daemon.sock");
    let db = dir.path().join("sessions.db");
    let script = vec![
        vec![Action::CallTool {
            call_id: "c1".to_string(),
            name: "write_file".to_string(),
            args: "{}".to_string(),
        }],
        vec![Action::Done {
            summary: "done".to_string(),
        }],
    ];
    let daemon = Daemon::new(config_with(&socket, &db, script));
    tokio::spawn(async move {
        let _ = daemon.run().await;
    });
    wait_for(&socket).await;

    let (mut reader, mut writer) = connect(&socket).await;
    let mut line = String::new();
    send(&mut writer, &ClientMsg::Attach { session: None }).await;
    let _ = read_server(&mut reader, &mut line).await;
    send(
        &mut writer,
        &ClientMsg::Prompt {
            text: "write".to_string(),
            skip_verify: false,
        },
    )
    .await;

    let id = read_until_approval(&mut reader, &mut line).await;
    send(
        &mut writer,
        &ClientMsg::Approve {
            id,
            yes: true,
            always: false,
        },
    )
    .await;
    let events = read_until_done(&mut reader, &mut line).await;
    assert!(events.iter().any(|message| matches!(
        message,
        ServerMsg::Event {
            event: SessionEvent::Done { .. },
            ..
        }
    )));
}

#[tokio::test]
async fn undo_restores_the_latest_checkpoint() {
    let dir = tempfile::tempdir().expect("tempdir");
    let socket = dir.path().join("daemon.sock");
    let db = dir.path().join("sessions.db");
    let recorder = Arc::new(RecordingCheckpointer::default());
    let mut config = config(&socket, &db);
    config.checkpointer = Some(recorder.clone());
    let daemon = Daemon::new(config);
    tokio::spawn(async move {
        let _ = daemon.run().await;
    });
    wait_for(&socket).await;

    let (mut reader, mut writer) = connect(&socket).await;
    let mut line = String::new();
    send(&mut writer, &ClientMsg::Attach { session: None }).await;
    let _ = read_server(&mut reader, &mut line).await;
    send(
        &mut writer,
        &ClientMsg::Prompt {
            text: "go".to_string(),
            skip_verify: false,
        },
    )
    .await;
    let _ = read_until_done(&mut reader, &mut line).await;

    send(&mut writer, &ClientMsg::Undo).await;
    let info = read_until_info(&mut reader, &mut line).await;
    assert!(info.contains("restored checkpoint"));
    assert!(!recorder.created.lock().expect("created").is_empty());
    assert_eq!(recorder.restored.lock().expect("restored").len(), 1);
}

#[tokio::test]
async fn delta_messages_reach_the_client() {
    let dir = tempfile::tempdir().expect("tempdir");
    let socket = dir.path().join("daemon.sock");
    let db = dir.path().join("sessions.db");
    let script = vec![vec![
        Action::Say {
            text: "hello".to_string(),
        },
        Action::Done {
            summary: "hello".to_string(),
        },
    ]];
    let daemon = Daemon::new(config_with(&socket, &db, script));
    tokio::spawn(async move {
        let _ = daemon.run().await;
    });
    wait_for(&socket).await;

    let (mut reader, mut writer) = connect(&socket).await;
    let mut line = String::new();
    send(&mut writer, &ClientMsg::Attach { session: None }).await;
    let _ = read_server(&mut reader, &mut line).await;
    send(
        &mut writer,
        &ClientMsg::Prompt {
            text: "say hello".to_string(),
            skip_verify: false,
        },
    )
    .await;

    let events = read_until_done(&mut reader, &mut line).await;
    assert!(events.iter().any(|message| matches!(
        message,
        ServerMsg::Delta { text } if text == "hello"
    )));
}

#[tokio::test]
async fn diff_returns_the_checkpoint_delta() {
    let dir = tempfile::tempdir().expect("tempdir");
    let socket = dir.path().join("daemon.sock");
    let db = dir.path().join("sessions.db");
    let recorder = Arc::new(RecordingCheckpointer::default());
    let mut config = config(&socket, &db);
    config.checkpointer = Some(recorder);
    let daemon = Daemon::new(config);
    tokio::spawn(async move {
        let _ = daemon.run().await;
    });
    wait_for(&socket).await;

    let (mut reader, mut writer) = connect(&socket).await;
    let mut line = String::new();
    send(&mut writer, &ClientMsg::Attach { session: None }).await;
    let _ = read_server(&mut reader, &mut line).await;
    send(
        &mut writer,
        &ClientMsg::Prompt {
            text: "go".to_string(),
            skip_verify: false,
        },
    )
    .await;
    let _ = read_until_done(&mut reader, &mut line).await;

    send(&mut writer, &ClientMsg::Diff).await;
    let diff = read_until_diff(&mut reader, &mut line).await;
    assert!(diff.contains("+two"));
}

#[tokio::test]
async fn lists_and_reads_workspace_files() {
    let dir = tempfile::tempdir().expect("tempdir");
    let socket = dir.path().join("daemon.sock");
    let db = dir.path().join("sessions.db");
    std::fs::write(dir.path().join("hello.txt"), "hi").expect("write");
    let daemon = Daemon::new(config_with(
        &socket,
        &db,
        vec![vec![Action::Done {
            summary: "done".to_string(),
        }]],
    ));
    tokio::spawn(async move {
        let _ = daemon.run().await;
    });
    wait_for(&socket).await;

    let (mut reader, mut writer) = connect(&socket).await;
    let mut line = String::new();
    send(&mut writer, &ClientMsg::Attach { session: None }).await;
    let _ = read_server(&mut reader, &mut line).await;

    send(
        &mut writer,
        &ClientMsg::ListDir {
            path: ".".to_string(),
        },
    )
    .await;
    let (path, entries) = read_until_dir(&mut reader, &mut line).await;
    assert_eq!(path, ".");
    assert!(entries.iter().any(|entry| entry.name == "hello.txt"));

    send(
        &mut writer,
        &ClientMsg::ReadFile {
            path: "hello.txt".to_string(),
        },
    )
    .await;
    let (path, content, truncated) = read_until_file(&mut reader, &mut line).await;
    assert_eq!(path, "hello.txt");
    assert_eq!(content, "hi");
    assert!(!truncated);
}

#[tokio::test]
async fn artifacts_include_the_checkpoint_diff() {
    let dir = tempfile::tempdir().expect("tempdir");
    let socket = dir.path().join("daemon.sock");
    let db = dir.path().join("sessions.db");
    let recorder = Arc::new(RecordingCheckpointer::default());
    let mut config = config_with(
        &socket,
        &db,
        vec![vec![Action::Done {
            summary: "done".to_string(),
        }]],
    );
    config.checkpointer = Some(recorder);
    let daemon = Daemon::new(config);
    tokio::spawn(async move {
        let _ = daemon.run().await;
    });
    wait_for(&socket).await;

    let (mut reader, mut writer) = connect(&socket).await;
    let mut line = String::new();
    send(&mut writer, &ClientMsg::Attach { session: None }).await;
    let _ = read_server(&mut reader, &mut line).await;
    send(
        &mut writer,
        &ClientMsg::Prompt {
            text: "go".to_string(),
            skip_verify: false,
        },
    )
    .await;
    let _ = read_until_done(&mut reader, &mut line).await;

    send(&mut writer, &ClientMsg::Artifacts).await;
    let items = read_until_artifacts(&mut reader, &mut line).await;
    assert!(items
        .iter()
        .any(|item| item.kind == "diff" && item.detail.contains("+two")));
}

#[tokio::test]
async fn settings_returns_profile_and_privacy() {
    let dir = tempfile::tempdir().expect("tempdir");
    let socket = dir.path().join("daemon.sock");
    let db = dir.path().join("sessions.db");
    let daemon = Daemon::new(config(&socket, &db));
    tokio::spawn(async move {
        let _ = daemon.run().await;
    });
    wait_for(&socket).await;

    let (mut reader, mut writer) = connect(&socket).await;
    let mut line = String::new();
    send(&mut writer, &ClientMsg::Attach { session: None }).await;
    let _ = read_server(&mut reader, &mut line).await;
    send(&mut writer, &ClientMsg::Settings).await;
    let settings = read_until_settings(&mut reader, &mut line).await;
    assert_eq!(settings.profile, "test");
    assert_eq!(settings.model, "test");
    assert!(settings
        .permissions
        .iter()
        .any(|line| line.contains("shell")));
    assert!(settings
        .privacy
        .iter()
        .any(|line| line.contains("no telemetry")));
}

#[tokio::test]
async fn audit_contains_approval_decision() {
    let dir = tempfile::tempdir().expect("tempdir");
    let socket = dir.path().join("daemon.sock");
    let db = dir.path().join("sessions.db");
    let script = vec![
        vec![Action::CallTool {
            call_id: "c1".to_string(),
            name: "write_file".to_string(),
            args: "{}".to_string(),
        }],
        vec![Action::Done {
            summary: "done".to_string(),
        }],
    ];
    let daemon = Daemon::new(config_with(&socket, &db, script));
    tokio::spawn(async move {
        let _ = daemon.run().await;
    });
    wait_for(&socket).await;

    let (mut reader, mut writer) = connect(&socket).await;
    let mut line = String::new();
    send(&mut writer, &ClientMsg::Attach { session: None }).await;
    let _ = read_server(&mut reader, &mut line).await;
    send(
        &mut writer,
        &ClientMsg::Prompt {
            text: "write".to_string(),
            skip_verify: false,
        },
    )
    .await;
    let id = read_until_approval(&mut reader, &mut line).await;
    send(
        &mut writer,
        &ClientMsg::Approve {
            id,
            yes: true,
            always: true,
        },
    )
    .await;
    let _ = read_until_done(&mut reader, &mut line).await;

    send(&mut writer, &ClientMsg::Audit).await;
    let entries = read_until_audit(&mut reader, &mut line).await;
    assert!(entries
        .iter()
        .any(|line| line.contains("approval write_file approved always=true")));
}

#[tokio::test]
async fn audit_contains_denied_approval() {
    let dir = tempfile::tempdir().expect("tempdir");
    let socket = dir.path().join("daemon.sock");
    let db = dir.path().join("sessions.db");
    let script = vec![
        vec![Action::CallTool {
            call_id: "c1".to_string(),
            name: "write_file".to_string(),
            args: "{}".to_string(),
        }],
        vec![Action::Done {
            summary: "done".to_string(),
        }],
    ];
    let daemon = Daemon::new(config_with(&socket, &db, script));
    tokio::spawn(async move {
        let _ = daemon.run().await;
    });
    wait_for(&socket).await;

    let (mut reader, mut writer) = connect(&socket).await;
    let mut line = String::new();
    send(&mut writer, &ClientMsg::Attach { session: None }).await;
    let _ = read_server(&mut reader, &mut line).await;
    send(
        &mut writer,
        &ClientMsg::Prompt {
            text: "write".to_string(),
            skip_verify: false,
        },
    )
    .await;
    let id = read_until_approval(&mut reader, &mut line).await;
    send(
        &mut writer,
        &ClientMsg::Approve {
            id,
            yes: false,
            always: false,
        },
    )
    .await;

    send(&mut writer, &ClientMsg::Audit).await;
    let entries = read_until_audit(&mut reader, &mut line).await;
    assert!(entries
        .iter()
        .any(|line| line.contains("approval write_file denied")));
}

#[tokio::test]
async fn self_view_returns_entity_state() {
    let dir = tempfile::tempdir().expect("tempdir");
    let socket = dir.path().join("daemon.sock");
    let db = dir.path().join("sessions.db");
    let daemon = Daemon::new(config(&socket, &db));
    tokio::spawn(async move {
        let _ = daemon.run().await;
    });
    wait_for(&socket).await;

    let (mut reader, mut writer) = connect(&socket).await;
    let mut line = String::new();
    send(&mut writer, &ClientMsg::Attach { session: None }).await;
    let _ = read_server(&mut reader, &mut line).await;
    send(&mut writer, &ClientMsg::SelfView).await;
    let text = read_until_self_view(&mut reader, &mut line).await;
    assert!(text.contains("drives"));
    assert!(text.contains("affect"));
    assert!(text.contains("posture"));
}

struct RecordingProvider {
    calls: Arc<Mutex<Vec<Vec<Message>>>>,
    tool_lists: Arc<Mutex<Vec<Vec<ToolSpec>>>>,
    thinkings: Arc<Mutex<Vec<orexis_core::Thinking>>>,
}

#[async_trait::async_trait]
impl Provider for RecordingProvider {
    async fn complete(
        &self,
        messages: &[Message],
        tools: &[ToolSpec],
        _step: usize,
        thinking: orexis_core::Thinking,
    ) -> PortResult<ProviderResponse> {
        self.calls.lock().expect("calls").push(messages.to_vec());
        self.tool_lists
            .lock()
            .expect("tool lists")
            .push(tools.to_vec());
        self.thinkings.lock().expect("thinkings").push(thinking);
        Ok(ProviderResponse {
            actions: vec![Action::Done {
                summary: "done".to_string(),
            }],
            raw_text: String::new(),
            usage: None,
        })
    }
}

struct FakeExternalTools {
    kind: &'static str,
    specs: Vec<ToolSpec>,
    failures: Vec<(String, String)>,
}

#[async_trait::async_trait]
impl ToolExecutor for FakeExternalTools {
    async fn execute(&self, name: &str, _args: &str) -> PortResult<Observation> {
        Ok(Observation {
            name: name.to_string(),
            output: "external ran".to_string(),
        })
    }
}

#[async_trait::async_trait]
impl ExternalTools for FakeExternalTools {
    fn kind(&self) -> &'static str {
        self.kind
    }

    async fn connect_all(&self) -> Vec<(String, String)> {
        self.failures.clone()
    }

    async fn specs(&self) -> Vec<ToolSpec> {
        self.specs.clone()
    }
}

#[tokio::test]
async fn identity_is_injected_only_on_the_first_turn() {
    let dir = tempfile::tempdir().expect("tempdir");
    let socket = dir.path().join("daemon.sock");
    let db = dir.path().join("sessions.db");
    let entity_db = dir.path().join("entity.db");
    let store = SqliteEntityStore::open(&entity_db).expect("entity");
    futures::executor::block_on(store.append(EntityEvent::TraitUpdate {
        trait_id: "t1".to_string(),
        statement: "prefers tests first".to_string(),
        evidence: vec![1],
        confidence: 0.8,
        last_seen_at: 1,
    }))
    .expect("trait");

    let calls = Arc::new(Mutex::new(Vec::new()));
    let mut config = config(&socket, &db);
    config.provider = Arc::new(RecordingProvider {
        calls: calls.clone(),
        tool_lists: Arc::new(Mutex::new(Vec::new())),
        thinkings: Arc::new(Mutex::new(Vec::new())),
    });
    config.entity = Arc::new(SqliteEntityFactory::new(entity_db));
    let daemon = Daemon::new(config);
    tokio::spawn(async move {
        let _ = daemon.run().await;
    });
    wait_for(&socket).await;

    let (mut reader, mut writer) = connect(&socket).await;
    let mut line = String::new();
    send(&mut writer, &ClientMsg::Attach { session: None }).await;
    let _ = read_server(&mut reader, &mut line).await;
    for prompt in ["first", "second"] {
        send(
            &mut writer,
            &ClientMsg::Prompt {
                text: prompt.to_string(),
                skip_verify: false,
            },
        )
        .await;
        let _ = read_until_done(&mut reader, &mut line).await;
        if prompt == "first" {
            futures::executor::block_on(store.append(EntityEvent::TraitUpdate {
                trait_id: "t2".to_string(),
                statement: "mid-session trait".to_string(),
                evidence: vec![2],
                confidence: 0.9,
                last_seen_at: 2,
            }))
            .expect("mid-session trait");
        }
    }

    let calls = calls.lock().expect("calls");
    assert_eq!(calls.len(), 2);
    let system = |index: usize| {
        calls[index]
            .iter()
            .find(|message| message.role == Role::System)
            .map(|message| message.text.clone())
            .unwrap_or_default()
    };
    assert!(system(0).contains("prefers tests first"));
    assert!(!system(1).contains("prefers tests first"));
    assert!(!system(1).contains("mid-session trait"));
}

#[tokio::test]
async fn self_view_writes_identity_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let socket = dir.path().join("daemon.sock");
    let db = dir.path().join("sessions.db");
    let entity_db = dir.path().join("entity.db");
    let identity_path = dir.path().join("IDENTITY.md");
    let store = SqliteEntityStore::open(&entity_db).expect("entity");
    futures::executor::block_on(store.append(EntityEvent::TraitUpdate {
        trait_id: "t1".to_string(),
        statement: "prefers tests first".to_string(),
        evidence: vec![1],
        confidence: 0.8,
        last_seen_at: 1,
    }))
    .expect("trait");

    let mut config = config(&socket, &db);
    config.identity_path = Some(identity_path.clone());
    let (_, mut reader, mut writer, mut line) = spawn_and_attach(config, &socket).await;
    send(&mut writer, &ClientMsg::SelfView).await;
    let _ = read_until_self_view(&mut reader, &mut line).await;

    let identity = std::fs::read_to_string(identity_path).expect("identity file");
    assert!(identity.contains("prefers tests first"));
}

#[tokio::test]
async fn self_view_reports_an_unwritable_identity_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let socket = dir.path().join("daemon.sock");
    let db = dir.path().join("sessions.db");
    let blocker = dir.path().join("blocker");
    std::fs::write(&blocker, "not a directory").expect("blocker");
    let identity_path = blocker.join("IDENTITY.md");

    let mut config = config(&socket, &db);
    config.identity_path = Some(identity_path);
    let (_, mut reader, mut writer, mut line) = spawn_and_attach(config, &socket).await;
    send(&mut writer, &ClientMsg::SelfView).await;
    let text = read_until_self_view(&mut reader, &mut line).await;
    assert!(text.contains("identity file not written"), "text: {text}");
}

#[tokio::test]
async fn entity_survives_model_switch() {
    let dir = tempfile::tempdir().expect("tempdir");
    let socket_a = dir.path().join("a.sock");
    let socket_b = dir.path().join("b.sock");
    let db = dir.path().join("sessions.db");
    let entity_db = dir.path().join("entity.db");
    let store = SqliteEntityStore::open(&entity_db).expect("entity");
    futures::executor::block_on(store.append(EntityEvent::TraitUpdate {
        trait_id: "t1".to_string(),
        statement: "model-agnostic trait".to_string(),
        evidence: vec![1],
        confidence: 0.8,
        last_seen_at: 1,
    }))
    .expect("trait");

    let mut config_a = config(&socket_a, &db);
    config_a.model_id = "model-a".to_string();
    let (handle, mut reader, mut writer, mut line) = spawn_and_attach(config_a, &socket_a).await;
    send(&mut writer, &ClientMsg::SelfView).await;
    let first = read_until_self_view(&mut reader, &mut line).await;
    assert!(first.contains("model-agnostic trait"));
    handle.abort();
    drop(writer);

    let mut config_b = config(&socket_b, &db);
    config_b.model_id = "model-b".to_string();
    let (_, mut reader, mut writer, mut line) = spawn_and_attach(config_b, &socket_b).await;
    send(&mut writer, &ClientMsg::SelfView).await;
    let second = read_until_self_view(&mut reader, &mut line).await;
    assert!(second.contains("model-agnostic trait"));
}

#[tokio::test]
async fn evolution_returns_the_ledger() {
    let dir = tempfile::tempdir().expect("tempdir");
    let socket = dir.path().join("daemon.sock");
    let db = dir.path().join("sessions.db");
    let corridor = orexis_adapters_evolution_git::GitCorridor::open(
        dir.path().join("corridor"),
        test_signer(),
    )
    .expect("corridor");
    std::fs::write(
        dir.path().join("corridor/ledger.jsonl"),
        "#1 apply\n#2 revert\n",
    )
    .expect("ledger");
    let mut config = config(&socket, &db);
    config.evolution = Some(Arc::new(corridor));
    let (_, mut reader, mut writer, mut line) = spawn_and_attach(config, &socket).await;
    send(&mut writer, &ClientMsg::Evolution).await;
    let entries = read_until_evolution(&mut reader, &mut line).await;
    assert_eq!(entries, vec!["#1 apply", "#2 revert"]);
}

#[tokio::test]
async fn evolution_shows_the_most_recent_entries() {
    let dir = tempfile::tempdir().expect("tempdir");
    let socket = dir.path().join("daemon.sock");
    let db = dir.path().join("sessions.db");
    let corridor = orexis_adapters_evolution_git::GitCorridor::open(
        dir.path().join("corridor"),
        test_signer(),
    )
    .expect("corridor");
    let ledger: String = (1..=600)
        .map(|seq| format!("entry-{seq}\n"))
        .collect::<Vec<_>>()
        .concat();
    std::fs::write(dir.path().join("corridor/ledger.jsonl"), ledger).expect("ledger");
    let mut config = config(&socket, &db);
    config.evolution = Some(Arc::new(corridor));
    let (_, mut reader, mut writer, mut line) = spawn_and_attach(config, &socket).await;
    send(&mut writer, &ClientMsg::Evolution).await;
    let entries = read_until_evolution(&mut reader, &mut line).await;
    assert_eq!(entries.len(), 500, "the overlay keeps the last 500");
    assert_eq!(entries[0], "entry-101", "oldest kept entry");
    assert_eq!(entries[499], "entry-600", "newest entry");
}

fn test_signer() -> Arc<dyn orexis_ports::Signer> {
    Arc::new(orexis_adapters_evolution_git::Ed25519Signer::from_seed(
        [7u8; 32],
    ))
}

#[tokio::test]
async fn mcp_tools_register_into_the_session() {
    let dir = tempfile::tempdir().expect("tempdir");
    let socket = dir.path().join("daemon.sock");
    let db = dir.path().join("sessions.db");

    let calls = Arc::new(Mutex::new(Vec::new()));
    let tool_lists = Arc::new(Mutex::new(Vec::new()));
    let mut config = config(&socket, &db);
    config.provider = Arc::new(RecordingProvider {
        calls: calls.clone(),
        tool_lists: tool_lists.clone(),
        thinkings: Arc::new(Mutex::new(Vec::new())),
    });
    config.external = vec![Arc::new(FakeExternalTools {
        kind: "mcp",
        specs: vec![ToolSpec {
            name: "mcp/mock/mock_tool".to_string(),
            description: "a mock tool".to_string(),
            schema: serde_json::json!({"type": "object"}),
        }],
        failures: Vec::new(),
    })];
    let (_, mut reader, mut writer, mut line) = spawn_and_attach(config, &socket).await;
    send(
        &mut writer,
        &ClientMsg::Prompt {
            text: "hi".to_string(),
            skip_verify: false,
        },
    )
    .await;
    let _ = read_until_done(&mut reader, &mut line).await;

    let tools = tool_lists.lock().expect("tool lists")[0].clone();
    assert!(tools.iter().any(|tool| tool.name == "mcp/mock/mock_tool"));
    let messages = calls.lock().expect("calls")[0].clone();
    assert!(
        messages[0].text.contains("untrusted data"),
        "system: {}",
        messages[0].text
    );
}

#[tokio::test]
async fn a_failed_mcp_server_is_a_visible_session_event() {
    let dir = tempfile::tempdir().expect("tempdir");
    let socket = dir.path().join("daemon.sock");
    let db = dir.path().join("sessions.db");

    let mut config = config(&socket, &db);
    config.external = vec![Arc::new(FakeExternalTools {
        kind: "mcp",
        specs: Vec::new(),
        failures: vec![("missing".to_string(), "spawn failed".to_string())],
    })];
    let (_, mut reader, _writer, mut line) = spawn_and_attach(config, &socket).await;

    // The failure is replayed during attach, before any prompt.
    let mut saw_error = false;
    for _ in 0..10 {
        match read_server(&mut reader, &mut line).await {
            ServerMsg::Event {
                event: SessionEvent::Error { message },
                ..
            } if message.contains("mcp source missing") => {
                saw_error = true;
                break;
            }
            _ => continue,
        }
    }
    assert!(saw_error, "the failed server was not visible");
}

#[tokio::test]
async fn utcp_tools_register_and_route_through_the_surface() {
    let dir = tempfile::tempdir().expect("tempdir");
    let socket = dir.path().join("daemon.sock");
    let db = dir.path().join("sessions.db");

    let mut config = config(&socket, &db);
    config.provider = Arc::new(ScriptedProvider::new(vec![vec![
        Action::CallTool {
            call_id: "c1".to_string(),
            name: "utcp/svc/echo".to_string(),
            args: r#"{"x":1}"#.to_string(),
        },
        Action::Done {
            summary: "done".to_string(),
        },
    ]]));
    config.policy =
        Arc::new(
            PermissionPolicy::standard().with_rules(vec![orexis_core::PermissionRule {
                tool: "utcp/*".to_string(),
                decision: orexis_core::Decision::Allow,
            }]),
        );
    config.external = vec![Arc::new(FakeExternalTools {
        kind: "utcp",
        specs: vec![ToolSpec {
            name: "utcp/svc/echo".to_string(),
            description: "a utcp tool".to_string(),
            schema: serde_json::json!({"type": "object"}),
        }],
        failures: Vec::new(),
    })];
    let (_, mut reader, mut writer, mut line) = spawn_and_attach(config, &socket).await;
    send(
        &mut writer,
        &ClientMsg::Prompt {
            text: "hi".to_string(),
            skip_verify: false,
        },
    )
    .await;
    let events = read_until_done(&mut reader, &mut line).await;
    let observed = events.iter().any(|message| {
        matches!(
            message,
            ServerMsg::Event {
                event: SessionEvent::Observation { name, .. },
                ..
            } if name == "utcp/svc/echo"
        )
    });
    assert!(observed, "the utcp call did not route through the surface");
}

#[tokio::test]
async fn the_posture_reaches_the_provider() {
    // The posture is the thinking level's source (ADR-0013): the turn
    // loop reads it from the entity snapshot and hands it to the
    // provider, which conditions the wire on it.
    let dir = tempfile::tempdir().expect("tempdir");
    let socket = dir.path().join("daemon.sock");
    let db = dir.path().join("sessions.db");
    let entity = Arc::new(SqliteEntityFactory::new(dir.path().join("entity.db")));
    {
        let refs = orexis_ports::EntityFactory::open(&*entity).expect("entity");
        refs.append
            .append(orexis_core::EntityEvent::PostureChange {
                posture: orexis_core::Posture {
                    thinking: orexis_core::Thinking::On,
                    ..orexis_core::Posture::default()
                },
                reason: "test".to_string(),
            })
            .await
            .expect("posture change");
    }
    let calls = Arc::new(Mutex::new(Vec::new()));
    let thinkings = Arc::new(Mutex::new(Vec::new()));
    let mut config = config(&socket, &db);
    config.entity = entity;
    config.provider = Arc::new(RecordingProvider {
        calls: calls.clone(),
        tool_lists: Arc::new(Mutex::new(Vec::new())),
        thinkings: thinkings.clone(),
    });
    let (_, mut reader, mut writer, mut line) = spawn_and_attach(config, &socket).await;
    send(
        &mut writer,
        &ClientMsg::Prompt {
            text: "hi".to_string(),
            skip_verify: false,
        },
    )
    .await;
    let _ = read_until_done(&mut reader, &mut line).await;
    let thinkings = thinkings.lock().expect("thinkings");
    assert_eq!(
        thinkings.last(),
        Some(&orexis_core::Thinking::On),
        "the provider saw the posture's thinking"
    );
}

#[tokio::test]
async fn team_prompt_seeds_a_plan_task() {
    let dir = tempfile::tempdir().expect("tempdir");
    let socket = dir.path().join("daemon.sock");
    let db = dir.path().join("sessions.db");
    let board = Arc::new(orexis_adapters_fake::FakeBoard::new(256));
    let mut config = config(&socket, &db);
    config.team = true;
    config.board = board.clone();
    let daemon = Daemon::new(config);
    tokio::spawn(async move {
        let _ = daemon.run().await;
    });
    wait_for(&socket).await;

    let (mut reader, mut writer) = connect(&socket).await;
    let mut line = String::new();
    send(&mut writer, &ClientMsg::Attach { session: None }).await;
    let ServerMsg::Attached { .. } = read_server(&mut reader, &mut line).await else {
        panic!("expected attached");
    };
    send(
        &mut writer,
        &ClientMsg::Prompt {
            text: "fix alpha and beta\nsecond line".to_string(),
            skip_verify: false,
        },
    )
    .await;
    let _ = read_until_done(&mut reader, &mut line).await;

    let tasks = orexis_ports::BoardStore::list(board.as_ref())
        .await
        .expect("board list");
    assert!(
        tasks
            .iter()
            .any(|task| task.required_strategy == "plan"
                && task.title.starts_with("fix alpha and beta")),
        "team prompt must seed a plan task on the board: {tasks:?}"
    );
}
