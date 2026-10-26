//! Local coding tools with a path sandbox.

pub mod checkpoints;
mod edit;
pub mod verify;

pub use checkpoints::GitCheckpointStore;
pub use verify::ShellVerifier;

use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use orexis_core::{Decision, Observation, PermissionPolicy, ToolSpec};
use orexis_ports::{ApprovalGate, PortError, PortResult, ToolExecutor};
use serde_json::{json, Value};

const MAX_READ_BYTES: usize = 64 * 1024;
const MAX_LIST_ENTRIES: usize = 200;
const MAX_SEARCH_RESULTS: usize = 100;
const MAX_SEARCH_FILE_BYTES: u64 = 1024 * 1024;
const MAX_SEARCH_DEPTH: usize = 20;

pub struct LocalTools {
    root: PathBuf,
}

impl LocalTools {
    pub fn new(root: impl Into<PathBuf>) -> PortResult<Self> {
        let root = root.into();
        let root = fs::canonicalize(&root)
            .map_err(|error| PortError(format!("workspace root: {error}")))?;
        Ok(Self { root })
    }

    pub fn specs() -> Vec<ToolSpec> {
        vec![
            ToolSpec {
                name: "read_file".to_string(),
                description: "Read a UTF-8 file inside the workspace".to_string(),
                schema: json!({
                    "type": "object",
                    "properties": { "path": { "type": "string" } },
                    "required": ["path"]
                }),
            },
            ToolSpec {
                name: "write_file".to_string(),
                description: "Write a UTF-8 file inside the workspace".to_string(),
                schema: json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string" },
                        "content": { "type": "string" }
                    },
                    "required": ["path", "content"]
                }),
            },
            ToolSpec {
                name: "edit_file".to_string(),
                description: "Replace one unique substring in a file".to_string(),
                schema: json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string" },
                        "old": { "type": "string" },
                        "new": { "type": "string" }
                    },
                    "required": ["path", "old", "new"]
                }),
            },
            ToolSpec {
                name: "list_dir".to_string(),
                description: "List a directory inside the workspace".to_string(),
                schema: json!({
                    "type": "object",
                    "properties": { "path": { "type": "string" } }
                }),
            },
            ToolSpec {
                name: "git_status".to_string(),
                description: "Read-only git status for the workspace".to_string(),
                schema: json!({ "type": "object", "properties": {} }),
            },
            ToolSpec {
                name: "git_diff".to_string(),
                description: "Read-only git diff for the workspace".to_string(),
                schema: json!({
                    "type": "object",
                    "properties": { "path": { "type": "string" } }
                }),
            },
            ToolSpec {
                name: "shell".to_string(),
                description: "Run a shell command at the workspace root".to_string(),
                schema: json!({
                    "type": "object",
                    "properties": {
                        "command": { "type": "string" },
                        "timeout_secs": { "type": "integer" },
                        "cwd": { "type": "string" }
                    },
                    "required": ["command"]
                }),
            },
            ToolSpec {
                name: "search".to_string(),
                description: "Search text files inside the workspace".to_string(),
                schema: json!({
                    "type": "object",
                    "properties": {
                        "query": { "type": "string" },
                        "path": { "type": "string" }
                    },
                    "required": ["query"]
                }),
            },
        ]
    }

    fn resolve(&self, path: &str) -> PortResult<PathBuf> {
        let candidate = if Path::new(path).is_absolute() {
            PathBuf::from(path)
        } else {
            self.root.join(path)
        };
        let normalized = normalize(&candidate);
        if !normalized.starts_with(&self.root) {
            return Err(PortError(format!("path escapes the workspace: {path}")));
        }
        if normalized.exists() {
            let canonical = fs::canonicalize(&normalized)
                .map_err(|error| PortError(format!("resolve {path}: {error}")))?;
            if !canonical.starts_with(&self.root) {
                return Err(PortError(format!("path escapes the workspace: {path}")));
            }
            return Ok(canonical);
        }
        let parent = normalized
            .parent()
            .ok_or_else(|| PortError(format!("invalid path: {path}")))?;
        let parent = fs::canonicalize(parent)
            .map_err(|error| PortError(format!("resolve parent of {path}: {error}")))?;
        if !parent.starts_with(&self.root) {
            return Err(PortError(format!("path escapes the workspace: {path}")));
        }
        let name = normalized
            .file_name()
            .ok_or_else(|| PortError(format!("invalid path: {path}")))?;
        Ok(parent.join(name))
    }

    fn read_file(&self, args: &Value) -> PortResult<Observation> {
        let path = self.resolve(arg_str(args, "path")?)?;
        let bytes = fs::read(&path)
            .map_err(|error| PortError(format!("read {}: {error}", path.display())))?;
        let limit = args
            .get("max_bytes")
            .and_then(Value::as_u64)
            .map(|value| value as usize)
            .unwrap_or(MAX_READ_BYTES)
            .min(MAX_READ_BYTES);
        let truncated = bytes.len() > limit;
        let text = String::from_utf8_lossy(&bytes[..bytes.len().min(limit)]).to_string();
        let output = if truncated {
            format!("{text}\n[truncated at {limit} bytes]")
        } else {
            text
        };
        Ok(Observation {
            name: "read_file".to_string(),
            output,
        })
    }

    fn write_file(&self, args: &Value) -> PortResult<Observation> {
        let path = self.resolve(arg_str(args, "path")?)?;
        let content = arg_str(args, "content")?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| PortError(format!("mkdir {}: {error}", parent.display())))?;
        }
        fs::write(&path, content)
            .map_err(|error| PortError(format!("write {}: {error}", path.display())))?;
        Ok(Observation {
            name: "write_file".to_string(),
            output: format!(
                "wrote {} bytes to {}",
                content.len(),
                path.strip_prefix(&self.root).unwrap_or(&path).display()
            ),
        })
    }

    fn edit_file(&self, args: &Value) -> PortResult<Observation> {
        let path = self.resolve(arg_str(args, "path")?)?;
        let old = arg_str(args, "old")?;
        let new = arg_str(args, "new")?;
        let text = fs::read_to_string(&path)
            .map_err(|error| PortError(format!("read {}: {error}", path.display())))?;
        let (updated, outcome) = crate::edit::apply(&text, old, new)?;
        fs::write(&path, updated)
            .map_err(|error| PortError(format!("write {}: {error}", path.display())))?;
        Ok(Observation {
            name: "edit_file".to_string(),
            output: format!(
                "edited {} ({})",
                path.strip_prefix(&self.root).unwrap_or(&path).display(),
                outcome.description()
            ),
        })
    }

    fn list_dir(&self, args: &Value) -> PortResult<Observation> {
        let path = self.resolve(args.get("path").and_then(Value::as_str).unwrap_or("."))?;
        let mut entries = Vec::new();
        let entries_iter = fs::read_dir(&path)
            .map_err(|error| PortError(format!("list {}: {error}", path.display())))?;
        for entry in entries_iter {
            let entry = entry.map_err(|error| PortError(error.to_string()))?;
            let name = entry.file_name().to_string_lossy().to_string();
            let file_type = entry
                .file_type()
                .map_err(|error| PortError(error.to_string()))?;
            if file_type.is_dir() {
                entries.push(format!("{name}/"));
            } else {
                let size = entry.metadata().map(|metadata| metadata.len()).unwrap_or(0);
                entries.push(format!("{name}  {size}"));
            }
            if entries.len() >= MAX_LIST_ENTRIES {
                break;
            }
        }
        entries.sort();
        Ok(Observation {
            name: "list_dir".to_string(),
            output: if entries.is_empty() {
                "(empty)".to_string()
            } else {
                entries.join("\n")
            },
        })
    }

    fn search(&self, args: &Value) -> PortResult<Observation> {
        let query = arg_str(args, "query")?;
        if query.is_empty() {
            return Err(PortError("query is empty".to_string()));
        }
        let start = self.resolve(args.get("path").and_then(Value::as_str).unwrap_or("."))?;
        let mut hits = Vec::new();
        self.walk(&start, query, 0, &mut hits)?;
        Ok(Observation {
            name: "search".to_string(),
            output: if hits.is_empty() {
                "no matches".to_string()
            } else {
                hits.join("\n")
            },
        })
    }

    fn walk(
        &self,
        path: &Path,
        query: &str,
        depth: usize,
        hits: &mut Vec<String>,
    ) -> PortResult<()> {
        if hits.len() >= MAX_SEARCH_RESULTS || depth > MAX_SEARCH_DEPTH {
            return Ok(());
        }
        let canonical = fs::canonicalize(path)
            .map_err(|error| PortError(format!("walk {}: {error}", path.display())))?;
        if !canonical.starts_with(&self.root) {
            return Ok(());
        }
        let metadata = fs::metadata(path)
            .map_err(|error| PortError(format!("stat {}: {error}", path.display())))?;
        if metadata.is_dir() {
            let mut entries: Vec<PathBuf> = fs::read_dir(path)
                .map_err(|error| PortError(format!("list {}: {error}", path.display())))?
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .collect();
            entries.sort();
            for entry in entries {
                let name = entry
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("");
                if matches!(name, ".git" | "target" | "node_modules") {
                    continue;
                }
                self.walk(&entry, query, depth + 1, hits)?;
                if hits.len() >= MAX_SEARCH_RESULTS {
                    break;
                }
            }
        } else if metadata.is_file() && metadata.len() <= MAX_SEARCH_FILE_BYTES {
            if let Ok(text) = fs::read_to_string(path) {
                for (index, line) in text.lines().enumerate() {
                    if line.contains(query) {
                        let relative = path.strip_prefix(&self.root).unwrap_or(path);
                        hits.push(format!(
                            "{}:{}: {}",
                            relative.display(),
                            index + 1,
                            line.trim()
                        ));
                        if hits.len() >= MAX_SEARCH_RESULTS {
                            return Ok(());
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn git_status(&self, _args: &Value) -> PortResult<Observation> {
        let output = Command::new("git")
            .arg("-C")
            .arg(&self.root)
            .arg("status")
            .arg("--porcelain")
            .output()
            .map_err(|error| PortError(format!("git status: {error}")))?;
        if !output.status.success() {
            return Err(PortError(format!(
                "git status: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
        Ok(Observation {
            name: "git_status".to_string(),
            output: String::from_utf8_lossy(&output.stdout).to_string(),
        })
    }

    fn git_diff(&self, args: &Value) -> PortResult<Observation> {
        let mut command = Command::new("git");
        command.arg("-C").arg(&self.root).arg("diff");
        if let Some(path) = args.get("path").and_then(Value::as_str) {
            command.arg("--").arg(path);
        }
        let output = command
            .output()
            .map_err(|error| PortError(format!("git diff: {error}")))?;
        if !output.status.success() {
            return Err(PortError(format!(
                "git diff: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
        let text = String::from_utf8_lossy(&output.stdout).to_string();
        let mut output_text = text.chars().take(SHELL_OUTPUT_CAP).collect::<String>();
        if output_text.chars().count() >= SHELL_OUTPUT_CAP {
            output_text.push_str("\n[truncated at 64KB]");
        }
        Ok(Observation {
            name: "git_diff".to_string(),
            output: output_text,
        })
    }

    async fn shell(&self, args: &Value) -> PortResult<Observation> {
        let command = arg_str(args, "command")?.to_string();
        let timeout = Duration::from_secs(
            args.get("timeout_secs")
                .and_then(Value::as_u64)
                .unwrap_or(120)
                .clamp(1, 600),
        );
        let cwd = self.resolve(args.get("cwd").and_then(Value::as_str).unwrap_or("."))?;
        if let Some(reason) = hardline_reason(&command) {
            return Err(PortError(format!("blocked command: {reason}")));
        }
        let output = tokio::task::spawn_blocking(move || run_shell(&cwd, &command, timeout))
            .await
            .map_err(|error| PortError(error.to_string()))??;
        Ok(Observation {
            name: "shell".to_string(),
            output,
        })
    }
}

#[async_trait]
impl ToolExecutor for LocalTools {
    async fn execute(&self, name: &str, args: &str) -> PortResult<Observation> {
        let value: Value = serde_json::from_str(args)
            .map_err(|error| PortError(format!("invalid args: {error}")))?;
        match name {
            "read_file" => self.read_file(&value),
            "write_file" => self.write_file(&value),
            "edit_file" => self.edit_file(&value),
            "list_dir" => self.list_dir(&value),
            "search" => self.search(&value),
            "shell" => self.shell(&value).await,
            "git_status" => self.git_status(&value),
            "git_diff" => self.git_diff(&value),
            other => Err(PortError(format!("unknown tool '{other}'"))),
        }
    }
}

const SHELL_OUTPUT_CAP: usize = 64 * 1024;

fn hardline_reason(command: &str) -> Option<&'static str> {
    const PATTERNS: &[(&str, &str)] = &[
        ("sudo ", "privilege escalation"),
        ("rm -rf /", "root deletion"),
        ("mkfs", "filesystem format"),
        ("dd if=/dev/", "raw device write"),
        (":(){", "fork bomb"),
        ("shutdown", "shutdown"),
        ("reboot", "reboot"),
    ];
    PATTERNS
        .iter()
        .find(|(pattern, _)| command.contains(pattern))
        .map(|(_, reason)| *reason)
}

pub(crate) fn run_shell(cwd: &Path, command: &str, timeout: Duration) -> PortResult<String> {
    let mut process = Command::new("sh");
    process
        .arg("-c")
        .arg(command)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    process.env_clear();
    for key in [
        "PATH",
        "HOME",
        "USER",
        "SHELL",
        "TERM",
        "LANG",
        "LC_ALL",
        "TMPDIR",
        "CARGO_HOME",
        "RUSTUP_HOME",
    ] {
        if let Ok(value) = std::env::var(key) {
            process.env(key, value);
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        process.process_group(0);
    }

    let mut child = process
        .spawn()
        .map_err(|error| PortError(format!("spawn shell: {error}")))?;
    let pid = child.id();
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let out_thread = std::thread::spawn(move || read_capped(stdout));
    let err_thread = std::thread::spawn(move || read_capped(stderr));

    let timed_out = wait_timeout(&mut child, timeout, pid);
    let stdout = out_thread.join().unwrap_or_default();
    let stderr = err_thread.join().unwrap_or_default();
    let code = child
        .try_wait()
        .ok()
        .flatten()
        .and_then(|status| status.code())
        .map(|code| code.to_string())
        .unwrap_or_else(|| "signal".to_string());

    let mut output = format!("exit {code}\n--- stdout ---\n{stdout}\n--- stderr ---\n{stderr}");
    if timed_out {
        output.push_str(&format!(
            "\n[timed out after {}s; process group killed]",
            timeout.as_secs()
        ));
    }
    Ok(output)
}

fn read_capped<R: Read>(reader: Option<R>) -> String {
    let Some(mut reader) = reader else {
        return String::new();
    };
    let mut kept = Vec::new();
    let mut buffer = [0u8; 8192];
    let mut truncated = false;
    loop {
        match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => {
                if kept.len() < SHELL_OUTPUT_CAP {
                    let take = (SHELL_OUTPUT_CAP - kept.len()).min(read);
                    kept.extend_from_slice(&buffer[..take]);
                    if take < read {
                        truncated = true;
                    }
                } else {
                    truncated = true;
                }
            }
            Err(_) => break,
        }
    }
    let mut text = String::from_utf8_lossy(&kept).to_string();
    if truncated {
        text.push_str("\n[truncated at 64KB]");
    }
    text
}

fn wait_timeout(child: &mut Child, timeout: Duration, pid: u32) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return false,
            Ok(None) => {}
            Err(_) => return false,
        }
        if Instant::now() >= deadline {
            kill_group(pid);
            let _ = child.wait();
            return true;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn kill_group(pid: u32) {
    #[cfg(unix)]
    {
        // SAFETY: kill(2) with a negative pid signals the child's process group.
        unsafe {
            libc::kill(-(pid as i32), libc::SIGKILL);
        }
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
    }
}

fn arg_str<'a>(value: &'a Value, key: &str) -> PortResult<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| PortError(format!("missing '{key}'")))
}

fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            Component::Prefix(prefix) => out.push(prefix.as_os_str()),
            Component::RootDir => out.push(Component::RootDir.as_os_str()),
            Component::Normal(part) => out.push(part),
        }
    }
    out
}

/// Wraps a tool executor with the permission ruleset and an approval gate.
pub struct GatedTools {
    inner: Arc<dyn ToolExecutor>,
    policy: Arc<PermissionPolicy>,
    gate: Arc<dyn ApprovalGate>,
}

impl GatedTools {
    pub fn new(
        inner: Arc<dyn ToolExecutor>,
        policy: Arc<PermissionPolicy>,
        gate: Arc<dyn ApprovalGate>,
    ) -> Self {
        Self {
            inner,
            policy,
            gate,
        }
    }
}

#[async_trait]
impl ToolExecutor for GatedTools {
    async fn execute(&self, name: &str, args: &str) -> PortResult<Observation> {
        match self.policy.decide(name) {
            Decision::Allow => self.inner.execute(name, args).await,
            Decision::Deny => Err(PortError(format!("denied by policy: {name}"))),
            Decision::Ask => {
                if self.gate.approve(name, args).await? {
                    self.inner.execute(name, args).await
                } else {
                    Err(PortError(format!("approval denied: {name}")))
                }
            }
        }
    }
}

pub struct AutoApprove;

#[async_trait]
impl ApprovalGate for AutoApprove {
    async fn approve(&self, _tool: &str, _args: &str) -> PortResult<bool> {
        Ok(true)
    }
}

pub struct DenyAll;

#[async_trait]
impl ApprovalGate for DenyAll {
    async fn approve(&self, _tool: &str, _args: &str) -> PortResult<bool> {
        Ok(false)
    }
}

#[cfg(test)]
mod gate_tests {
    use super::*;
    use futures::executor::block_on;

    struct StubTools;

    #[async_trait]
    impl ToolExecutor for StubTools {
        async fn execute(&self, name: &str, _args: &str) -> PortResult<Observation> {
            Ok(Observation {
                name: name.to_string(),
                output: "ran".to_string(),
            })
        }
    }

    fn gated(policy: PermissionPolicy, gate: Arc<dyn ApprovalGate>) -> GatedTools {
        GatedTools::new(Arc::new(StubTools), Arc::new(policy), gate)
    }

    #[test]
    fn allow_passes() {
        let tools = gated(PermissionPolicy::standard(), Arc::new(AutoApprove));
        let observation = block_on(tools.execute("read_file", "{}")).expect("allow");
        assert_eq!(observation.output, "ran");
    }

    #[test]
    fn deny_blocks() {
        let tools = gated(PermissionPolicy::plan(), Arc::new(AutoApprove));
        assert!(block_on(tools.execute("write_file", "{}")).is_err());
    }

    #[test]
    fn ask_uses_the_gate() {
        let tools = gated(PermissionPolicy::standard(), Arc::new(AutoApprove));
        assert!(block_on(tools.execute("shell", "{}")).is_ok());

        let tools = gated(PermissionPolicy::standard(), Arc::new(DenyAll));
        assert!(block_on(tools.execute("shell", "{}")).is_err());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::executor::block_on;

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
}
