//! Unit tests for the local tool surface.

use std::sync::Arc;

use async_trait::async_trait;
use futures::executor::block_on;
use orexis_adapters_tools::LocalTools;
use orexis_core::Observation;
use orexis_ports::{Diagnostic, Lsp, PortResult, ToolExecutor};

fn setup() -> (tempfile::TempDir, LocalTools) {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("a.txt"), "hello\nworld\n").expect("write");
    std::fs::create_dir(dir.path().join("dir")).expect("mkdir");
    std::fs::write(dir.path().join("dir/b.txt"), "needle\n").expect("write");
    let tools = LocalTools::new(dir.path()).expect("tools");
    (dir, tools)
}

fn run(tools: &LocalTools, name: &str, args: &str) -> PortResult<Observation> {
    block_on(tools.execute(name, args))
}

#[test]
fn read_write_edit_round_trip() {
    let (_dir, tools) = setup();
    let read = run(&tools, "read_file", r#"{"path":"a.txt"}"#).expect("read");
    assert!(read.output.contains("hello"));

    run(&tools, "write_file", r#"{"path":"c.txt","content":"new"}"#).expect("write");
    let written = run(&tools, "read_file", r#"{"path":"c.txt"}"#).expect("read");
    assert_eq!(written.output, "new");

    run(
        &tools,
        "edit_file",
        r#"{"path":"a.txt","old":"world","new":"there"}"#,
    )
    .expect("edit");
    let edited = run(&tools, "read_file", r#"{"path":"a.txt"}"#).expect("read");
    assert!(edited.output.contains("there"));
    assert!(!edited.output.contains("world"));
}

#[test]
fn edit_rejects_absent_and_ambiguous() {
    let (_dir, tools) = setup();
    assert!(run(
        &tools,
        "edit_file",
        r#"{"path":"a.txt","old":"nope","new":"x"}"#
    )
    .is_err());
    assert!(run(
        &tools,
        "edit_file",
        r#"{"path":"a.txt","old":"l","new":"x"}"#
    )
    .is_err());
}

#[test]
fn list_and_search_work() {
    let (_dir, tools) = setup();
    let listed = run(&tools, "list_dir", r#"{"path":"."}"#).expect("list");
    assert!(listed.output.contains("a.txt"));
    assert!(listed.output.contains("dir/"));

    let found = run(&tools, "search", r#"{"query":"needle"}"#).expect("search");
    assert!(found.output.contains("dir/b.txt:1"));
}

#[test]
fn rejects_relative_escape() {
    let (_dir, tools) = setup();
    assert!(run(&tools, "read_file", r#"{"path":"../outside.txt"}"#).is_err());
}

#[test]
fn rejects_absolute_outside() {
    let (_dir, tools) = setup();
    assert!(run(&tools, "read_file", r#"{"path":"/etc/passwd"}"#).is_err());
}

#[cfg(unix)]
#[test]
fn rejects_symlink_escape() {
    let outside = tempfile::tempdir().expect("outside");
    std::fs::write(outside.path().join("secret"), "secret").expect("write");
    let (dir, tools) = setup();
    std::os::unix::fs::symlink(outside.path(), dir.path().join("link")).expect("symlink");
    assert!(run(&tools, "read_file", r#"{"path":"link/secret"}"#).is_err());
}

#[test]
fn read_truncates_at_the_cap() {
    let (dir, tools) = setup();
    let big = "x".repeat(70 * 1024);
    std::fs::write(dir.path().join("big.txt"), big).expect("write");
    let read = run(&tools, "read_file", r#"{"path":"big.txt"}"#).expect("read");
    assert!(read.output.contains("[truncated at 65536 bytes]"));
}

#[test]
fn unknown_tool_is_an_error() {
    let (_dir, tools) = setup();
    assert!(run(&tools, "nope", "{}").is_err());
}

#[tokio::test]
async fn shell_runs_in_the_root() {
    let (_dir, tools) = setup();
    let out = tools
        .execute("shell", r#"{"command":"echo hello"}"#)
        .await
        .expect("shell");
    assert!(out.output.contains("hello"));
    assert!(out.output.contains("exit 0"));
}

#[tokio::test]
async fn shell_reports_nonzero_exit() {
    let (_dir, tools) = setup();
    let out = tools
        .execute("shell", r#"{"command":"exit 3"}"#)
        .await
        .expect("shell");
    assert!(out.output.contains("exit 3"));
}

#[tokio::test]
async fn shell_times_out_and_kills_the_group() {
    let (_dir, tools) = setup();
    let start = std::time::Instant::now();
    let out = tools
        .execute("shell", r#"{"command":"sleep 5","timeout_secs":1}"#)
        .await
        .expect("shell");
    assert!(out.output.contains("timed out"));
    assert!(start.elapsed() < std::time::Duration::from_secs(4));
}

#[tokio::test]
async fn shell_caps_output() {
    let (_dir, tools) = setup();
    let out = tools
        .execute("shell", r#"{"command":"python3 -c 'print(\"x\"*100000)'"}"#)
        .await
        .expect("shell");
    assert!(out.output.contains("[truncated at 64KB]"));
}

#[tokio::test]
async fn shell_blocks_hardline_commands() {
    let (_dir, tools) = setup();
    assert!(tools
        .execute("shell", r#"{"command":"sudo rm -rf /"}"#)
        .await
        .is_err());
}

#[tokio::test]
async fn shell_scrubs_the_environment() {
    std::env::set_var("OREXIS_TEST_SECRET", "leak");
    let (_dir, tools) = setup();
    let out = tools
        .execute("shell", r#"{"command":"env"}"#)
        .await
        .expect("shell");
    std::env::remove_var("OREXIS_TEST_SECRET");
    assert!(!out.output.contains("OREXIS_TEST_SECRET"));
}

#[tokio::test]
async fn shell_rejects_cwd_escape() {
    let (_dir, tools) = setup();
    assert!(tools
        .execute("shell", r#"{"command":"pwd","cwd":"../"}"#)
        .await
        .is_err());
}

fn git_repo(dir: &std::path::Path) {
    let run_git = |args: &[&str]| {
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
    run_git(&["init", "-q"]);
    run_git(&["add", "-A"]);
    run_git(&["commit", "-q", "-m", "init"]);
}

#[test]
fn git_status_and_diff_are_read_only() {
    let (dir, tools) = setup();
    git_repo(dir.path());
    std::fs::write(dir.path().join("a.txt"), "hello\nthere\n").expect("write");

    let status = run(&tools, "git_status", "{}").expect("status");
    assert!(status.output.contains("a.txt"));

    let diff = run(&tools, "git_diff", "{}").expect("diff");
    assert!(diff.output.contains("-world"));
    assert!(diff.output.contains("+there"));
}

#[test]
fn git_status_fails_outside_a_repo() {
    let (_dir, tools) = setup();
    assert!(run(&tools, "git_status", "{}").is_err());
}

struct RecordingLsp;

#[async_trait]
impl Lsp for RecordingLsp {
    async fn diagnostics(&self, path: &str, _content: &str) -> PortResult<Vec<Diagnostic>> {
        Ok(vec![Diagnostic {
            path: path.to_string(),
            line: 0,
            column: 0,
            severity: "error".to_string(),
            message: "boom".to_string(),
        }])
    }
}

#[tokio::test]
async fn edit_attaches_diagnostics() {
    let (dir, _tools) = setup();
    let tools = LocalTools::with_lsp(dir.path(), Arc::new(RecordingLsp)).expect("tools");
    let out = tools
        .execute(
            "edit_file",
            r#"{"path":"a.txt","old":"world","new":"there"}"#,
        )
        .await
        .expect("edit");
    assert!(out.output.contains("diagnostics"));
    assert!(out.output.contains("boom"));
}
