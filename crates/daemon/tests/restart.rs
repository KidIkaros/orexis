//! Integration: SIGKILL the daemon, restart it, and replay the session.

use std::path::Path;
use std::time::Duration;

use orexis_core::SessionEvent;
use orexis_daemon::{ClientMsg, ServerMsg};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::unix::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::UnixStream;
use tokio::process::{Child, Command};
use tokio::time::timeout;

fn spawn(socket: &Path, db: &Path) -> Child {
    Command::new(env!("CARGO_BIN_EXE_serve"))
        .env("OREXIS_FAKE", "1")
        .env("OREXIS_SOCKET", socket)
        .env("OREXIS_DB", db)
        .kill_on_drop(true)
        .spawn()
        .expect("spawn serve")
}

async fn connect(path: &Path) -> UnixStream {
    for _ in 0..300 {
        if let Ok(stream) = UnixStream::connect(path).await {
            return stream;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("daemon did not start");
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

#[tokio::test]
async fn kill_and_restart_preserves_the_session() {
    let dir = tempfile::tempdir().expect("tempdir");
    let socket = dir.path().join("daemon.sock");
    let db = dir.path().join("sessions.db");

    let mut child = spawn(&socket, &db);
    let stream = connect(&socket).await;
    let (read_half, mut writer) = stream.into_split();
    let mut reader = BufReader::new(read_half);
    let mut line = String::new();

    send(&mut writer, &ClientMsg::Attach { session: None }).await;
    let ServerMsg::Attached { session_id } = read_server(&mut reader, &mut line).await else {
        panic!("expected attached");
    };
    send(
        &mut writer,
        &ClientMsg::Prompt {
            text: "before the kill".to_string(),
        },
    )
    .await;
    let _ = read_until_done(&mut reader, &mut line).await;

    child.kill().await.expect("kill");
    let _ = child.wait().await;
    drop(writer);

    let mut child = spawn(&socket, &db);
    let stream = connect(&socket).await;
    let (read_half, mut writer) = stream.into_split();
    let mut reader = BufReader::new(read_half);
    let mut line = String::new();

    send(&mut writer, &ClientMsg::ListSessions).await;
    let listed = read_server(&mut reader, &mut line).await;
    let ServerMsg::Sessions { sessions } = listed else {
        panic!("expected sessions");
    };
    assert!(sessions.iter().any(|session| session.id == session_id));

    send(
        &mut writer,
        &ClientMsg::Attach {
            session: Some(session_id),
        },
    )
    .await;
    let _ = read_server(&mut reader, &mut line).await;
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

    child.kill().await.expect("kill");
    let _ = child.wait().await;
}
