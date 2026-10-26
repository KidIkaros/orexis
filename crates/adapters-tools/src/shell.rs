//! The sandboxed shell tool.

use std::io::Read;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use orexis_core::Observation;
use orexis_ports::{PortError, PortResult};
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
