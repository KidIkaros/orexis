//! Integration: attach, prompt, detach, reattach, replay, and two sessions.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use orexis_adapters_codec::NativeCodec;
use orexis_adapters_fake::{EchoTools, ScriptedProvider};
use orexis_adapters_session_sqlite::SqliteSessionFactory;
use orexis_core::{Action, PermissionPolicy, SessionEvent};
use orexis_daemon::{ClientMsg, Daemon, DaemonConfig, ServerMsg};
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
        sessions: Arc::new(SqliteSessionFactory::new(db.to_path_buf())),
        provider: Arc::new(ScriptedProvider::new(script)),
        tools: Arc::new(EchoTools),
        codec: Arc::new(NativeCodec),
        policy: Arc::new(PermissionPolicy::standard()),
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
        },
    )
    .await;
    send(
        &mut writer_b,
        &ClientMsg::Prompt {
            text: "b".to_string(),
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
        },
    )
    .await;

    let id = read_until_approval(&mut reader, &mut line).await;
    send(&mut writer, &ClientMsg::Approve { id, yes: true }).await;
    let events = read_until_done(&mut reader, &mut line).await;
    assert!(events.iter().any(|message| matches!(
        message,
        ServerMsg::Event {
            event: SessionEvent::Done { .. },
            ..
        }
    )));
}
