//! Gate: a real repository task through the daemon — read, edit, test, diff.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use orexis_adapters_codec::NativeCodec;
use orexis_adapters_fake::ScriptedProvider;
use orexis_adapters_session_sqlite::SqliteSessionFactory;
use orexis_adapters_tools::{GitCheckpointStore, LocalTools, ShellVerifier};
use orexis_core::{Action, Decision, PermissionPolicy, PermissionRule, SessionEvent};
use orexis_daemon::{ClientMsg, Daemon, DaemonConfig, ServerMsg};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::unix::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::UnixStream;
use tokio::time::timeout;

fn write_project(dir: &Path) {
    std::fs::create_dir_all(dir.join("src")).expect("src");
    std::fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("cargo");
    std::fs::write(
        dir.join("src/lib.rs"),
        "pub fn add(a: i32, b: i32) -> i32 { a - b }\n\n#[test]\nfn adds() { assert_eq!(add(2, 3), 5); }\n",
    )
    .expect("lib");

    let git = |args: &[&str]| {
        let output = std::process::Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_AUTHOR_NAME", "test")
            .env("GIT_AUTHOR_EMAIL", "test@localhost")
            .env("GIT_COMMITTER_NAME", "test")
            .env("GIT_COMMITTER_EMAIL", "test@localhost")
            .output()
            .expect("git");
        assert!(
            output.status.success(),
            "git {:?}: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
    };
    git(&["init", "-q"]);
    git(&["add", "-A"]);
    git(&["commit", "-q", "-m", "init"]);
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

async fn connect(path: &Path) -> (BufReader<OwnedReadHalf>, OwnedWriteHalf) {
    let stream = UnixStream::connect(path).await.expect("connect");
    let (read_half, write_half) = stream.into_split();
    (BufReader::new(read_half), write_half)
}

async fn send(writer: &mut OwnedWriteHalf, message: &ClientMsg) {
    let mut line = serde_json::to_string(message).expect("client msg");
    line.push('\n');
    writer.write_all(line.as_bytes()).await.expect("write");
    writer.flush().await.expect("flush");
}

async fn read_server(reader: &mut BufReader<OwnedReadHalf>, line: &mut String) -> ServerMsg {
    line.clear();
    let read = timeout(Duration::from_secs(120), reader.read_line(line))
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
async fn a_real_repository_task_reads_edits_tests_and_diffs() {
    let dir = tempfile::tempdir().expect("tempdir");
    let project = dir.path();
    write_project(project);
    let socket = project.join("daemon.sock");
    let db = project.join("sessions.db");

    let script = vec![
        vec![Action::CallTool {
            call_id: "c1".to_string(),
            name: "read_file".to_string(),
            args: r#"{"path":"src/lib.rs"}"#.to_string(),
        }],
        vec![Action::CallTool {
            call_id: "c2".to_string(),
            name: "edit_file".to_string(),
            args: r#"{"path":"src/lib.rs","old":"a - b","new":"a + b"}"#.to_string(),
        }],
        vec![Action::CallTool {
            call_id: "c3".to_string(),
            name: "shell".to_string(),
            args: r#"{"command":"cargo test","timeout_secs":120}"#.to_string(),
        }],
        vec![Action::CallTool {
            call_id: "c4".to_string(),
            name: "git_diff".to_string(),
            args: "{}".to_string(),
        }],
        vec![Action::Done {
            summary: "fixed add".to_string(),
        }],
    ];

    let policy = PermissionPolicy::standard().with_rules(vec![
        PermissionRule {
            tool: "edit_file".to_string(),
            decision: Decision::Allow,
        },
        PermissionRule {
            tool: "shell".to_string(),
            decision: Decision::Allow,
        },
    ]);

    let config = DaemonConfig {
        socket: socket.clone(),
        sessions: Arc::new(SqliteSessionFactory::new(db)),
        provider: Arc::new(ScriptedProvider::new(script)),
        tools: Arc::new(LocalTools::new(project).expect("tools")),
        codec: Arc::new(NativeCodec),
        policy: Arc::new(policy),
        checkpointer: Some(Arc::new(
            GitCheckpointStore::new(project).expect("checkpoints"),
        )),
        verifier: Some(Arc::new(
            ShellVerifier::new(project, "cargo test").expect("verifier"),
        )),
        system_prompt: String::new(),
        context_tools: LocalTools::specs(),
        context_files: Vec::new(),
        context_limit: 32768,
        max_steps: 8,
        generation_slots: 1,
    };
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
            text: "fix add".to_string(),
            skip_verify: false,
        },
    )
    .await;

    let events = read_until_done(&mut reader, &mut line).await;
    let text = std::fs::read_to_string(project.join("src/lib.rs")).expect("read");
    assert!(text.contains("a + b"));

    let observations: Vec<String> = events
        .iter()
        .filter_map(|message| match message {
            ServerMsg::Event {
                event: SessionEvent::Observation { output, .. },
                ..
            } => Some(output.clone()),
            _ => None,
        })
        .collect();
    assert!(observations
        .iter()
        .any(|output| output.contains("test result: ok")));
    assert!(observations
        .iter()
        .any(|output| output.contains("a + b") && output.contains("-")));

    assert!(events.iter().any(|message| matches!(
        message,
        ServerMsg::Event {
            event: SessionEvent::Verify { passed: true, .. },
            ..
        }
    )));
    assert!(events.iter().any(|message| matches!(
        message,
        ServerMsg::Event {
            event: SessionEvent::Done { summary },
            ..
        } if summary == "fixed add"
    )));
}
