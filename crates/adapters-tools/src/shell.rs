//! The sandboxed shell tool.

use std::io::Read;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use orexis_core::Observation;
use orexis_ports::{CommandOutput, CommandRunner, PortError, PortResult};
use serde_json::Value;

use crate::{arg_str, LocalTools};

impl LocalTools {
    pub(crate) async fn shell(&self, args: &Value) -> PortResult<Observation> {
        let command = arg_str(args, "command")?.to_string();
        let timeout = Duration::from_secs(
            args.get("timeout_secs")
                .and_then(Value::as_u64)
                .unwrap_or(120)
                .clamp(1, 600),
        );
        let cwd = self.resolve(args.get("cwd").and_then(Value::as_str).unwrap_or("."))?;
        let root = self.root.clone();
        if let Some(reason) = hardline_reason(&command) {
            return Err(PortError(format!("blocked command: {reason}")));
        }
        let output =
            tokio::task::spawn_blocking(move || run_shell_in(&root, &cwd, &command, timeout))
                .await
                .map_err(|error| PortError(error.to_string()))??;
        Ok(Observation {
            name: "shell".to_string(),
            output,
        })
    }
}

/// `CommandRunner` for transport shells (UTCP CLI providers): argv-exact —
/// no shell interpretation of arguments — inside the same sandbox as the
/// `shell` tool (env scrub, workspace cwd, process group, caps).
#[async_trait::async_trait]
impl CommandRunner for LocalTools {
    async fn run(
        &self,
        program: &str,
        args: &[String],
        timeout: Duration,
    ) -> PortResult<CommandOutput> {
        let joined = format!("{program} {}", args.join(" "));
        if let Some(reason) = hardline_reason(&joined).or_else(|| blocked_program(program)) {
            return Err(PortError(format!("blocked command: {reason}")));
        }
        let cwd = self.root.clone();
        let program = program.to_string();
        let args = args.to_vec();
        tokio::task::spawn_blocking(move || run_program(&cwd, &program, &args, timeout))
            .await
            .map_err(|error| PortError(error.to_string()))?
    }
}

/// Programs whose argv form the joined-string patterns miss (a bare
/// `sudo` has no trailing space). The safety floor applies to argv too.
fn blocked_program(program: &str) -> Option<&'static str> {
    let basename = Path::new(program)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(program);
    const BLOCKED: &[(&str, &str)] = &[
        ("sudo", "privilege escalation"),
        ("su", "privilege escalation"),
        ("doas", "privilege escalation"),
        ("shutdown", "shutdown"),
        ("reboot", "reboot"),
        ("halt", "shutdown"),
        ("poweroff", "shutdown"),
    ];
    if basename.starts_with("mkfs") {
        return Some("filesystem format");
    }
    BLOCKED
        .iter()
        .find(|(name, _)| basename == *name)
        .map(|(_, reason)| *reason)
}

/// Build the Linux `bwrap` (bubblewrap) argument prefix that mounts `/`
/// read-only, isolates `/tmp`, mounts only `root` read-write, optionally
/// unshares the network namespace, and binds child lifetime to the parent.
pub fn bwrap_prefix(root: &Path, cwd: &Path, isolate_net: bool) -> Vec<String> {
    let root_str = root.to_string_lossy().into_owned();
    let cwd_str = cwd.to_string_lossy().into_owned();
    let mut args = vec![
        "--ro-bind".to_string(),
        "/".to_string(),
        "/".to_string(),
        "--dev".to_string(),
        "/dev".to_string(),
        "--proc".to_string(),
        "/proc".to_string(),
        "--tmpfs".to_string(),
        "/tmp".to_string(),
        "--bind".to_string(),
        root_str.clone(),
        root_str,
        "--die-with-parent".to_string(),
        "--chdir".to_string(),
        cwd_str,
    ];
    if isolate_net {
        args.push("--unshare-net".to_string());
    }
    args.push("--".to_string());
    args
}

/// True when `bwrap` is installed, unprivileged user namespaces are allowed
/// on this host, and `OREXIS_NO_SANDBOX` is not set.
pub fn bwrap_available() -> bool {
    if std::env::var("OREXIS_NO_SANDBOX").is_ok() {
        return false;
    }
    static AVAILABLE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *AVAILABLE.get_or_init(|| {
        Command::new("bwrap")
            .args([
                "--ro-bind",
                "/",
                "/",
                "--dev",
                "/dev",
                "--proc",
                "/proc",
                "--die-with-parent",
                "--",
                "true",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    })
}

pub(crate) fn run_program(
    cwd: &Path,
    program: &str,
    args: &[String],
    timeout: Duration,
) -> PortResult<CommandOutput> {
    let mut process = if bwrap_available() {
        let mut cmd = Command::new("bwrap");
        let isolate_net = std::env::var("OREXIS_SANDBOX_NET").is_err();
        cmd.args(bwrap_prefix(cwd, cwd, isolate_net));
        cmd.arg(program);
        cmd.args(args);
        cmd
    } else {
        let mut cmd = Command::new(program);
        cmd.args(args);
        cmd
    };
    configure(&mut process, cwd);
    let child = process
        .spawn()
        .map_err(|error| PortError(format!("spawn {program}: {error}")))?;
    Ok(collect(child, timeout))
}

pub(crate) const SHELL_OUTPUT_CAP: usize = 64 * 1024;

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
    run_shell_in(cwd, cwd, command, timeout)
}

pub(crate) fn run_shell_in(
    root: &Path,
    cwd: &Path,
    command: &str,
    timeout: Duration,
) -> PortResult<String> {
    let mut process = if bwrap_available() {
        let mut cmd = Command::new("bwrap");
        let isolate_net = std::env::var("OREXIS_SANDBOX_NET").is_err();
        cmd.args(bwrap_prefix(root, cwd, isolate_net));
        cmd.arg("sh").arg("-c").arg(command);
        cmd
    } else {
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(command);
        cmd
    };
    configure(&mut process, cwd);
    let child = process
        .spawn()
        .map_err(|error| PortError(format!("spawn shell: {error}")))?;
    let output = collect(child, timeout);
    let code = output
        .code
        .map(|code| code.to_string())
        .unwrap_or_else(|| "signal".to_string());

    let mut text = format!(
        "exit {code}\n--- stdout ---\n{}\n--- stderr ---\n{}",
        output.stdout, output.stderr
    );
    if output.timed_out {
        text.push_str(&format!(
            "\n[timed out after {}s; process group killed]",
            timeout.as_secs()
        ));
    }
    Ok(text)
}

/// The sandbox posture shared by `sh -c` and argv-exact runs.
fn configure(process: &mut Command, cwd: &Path) {
    process
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
}

fn collect(mut child: Child, timeout: Duration) -> CommandOutput {
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
        .and_then(|status| status.code());
    CommandOutput {
        code,
        stdout,
        stderr,
        timed_out,
    }
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
