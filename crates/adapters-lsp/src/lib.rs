//! Minimal stdio LSP client with a persistent server per workspace.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use orexis_ports::{Diagnostic, Lsp, PortResult};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex;

struct Session {
    _child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    versions: HashMap<String, i32>,
}

pub struct StdioLsp {
    root: PathBuf,
    command: Vec<String>,
    timeout: Duration,
    session: Mutex<Option<Session>>,
    spawns: AtomicUsize,
}

impl StdioLsp {
    pub fn new(root: impl Into<PathBuf>, command: impl Into<String>) -> Self {
        Self {
            root: root.into(),
            command: vec![command.into()],
            timeout: Duration::from_secs(5),
            session: Mutex::new(None),
            spawns: AtomicUsize::new(0),
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Number of server processes started; a persistent client starts one.
    pub fn spawns(&self) -> usize {
        self.spawns.load(Ordering::SeqCst)
    }

    async fn start(&self) -> std::io::Result<Session> {
        let mut command = Command::new(&self.command[0]);
        command
            .args(&self.command[1..])
            .current_dir(&self.root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        let mut child = command.spawn()?;
        let Some(mut stdin) = child.stdin.take() else {
            return Err(std::io::Error::other("missing lsp stdin"));
        };
        let Some(stdout) = child.stdout.take() else {
            return Err(std::io::Error::other("missing lsp stdout"));
        };
        let mut stdout = BufReader::new(stdout);

        write_message(
            &mut stdin,
            &json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "processId": null,
                    "rootUri": format!("file://{}", self.root.display()),
                    "capabilities": {}
                }
            }),
        )
        .await?;
        loop {
            let Some(message) = read_message(&mut stdout).await? else {
                return Err(std::io::Error::other("lsp closed during initialize"));
            };
            if message.get("id").and_then(Value::as_u64) == Some(1) {
                break;
            }
        }
        write_message(
            &mut stdin,
            &json!({ "jsonrpc": "2.0", "method": "initialized", "params": {} }),
        )
        .await?;

        Ok(Session {
            _child: child,
            stdin,
            stdout,
            versions: HashMap::new(),
        })
    }
}

#[async_trait]
impl Lsp for StdioLsp {
    async fn diagnostics(&self, path: &str, content: &str) -> PortResult<Vec<Diagnostic>> {
        let uri = format!("file://{}", self.root.join(path).display());
        let mut guard = self.session.lock().await;

        if guard.is_none() {
            match tokio::time::timeout(self.timeout, self.start()).await {
                Ok(Ok(session)) => {
                    *guard = Some(session);
                    self.spawns.fetch_add(1, Ordering::SeqCst);
                }
                _ => return Ok(Vec::new()),
            }
        }

        let outcome = {
            let Some(session) = guard.as_mut() else {
                return Ok(Vec::new());
            };
            let version = session.versions.get(&uri).copied().unwrap_or(0) + 1;
            session.versions.insert(uri.clone(), version);
            let message = if version == 1 {
                json!({
                    "jsonrpc": "2.0",
                    "method": "textDocument/didOpen",
                    "params": {
                        "textDocument": {
                            "uri": uri,
                            "languageId": "rust",
                            "version": version,
                            "text": content
                        }
                    }
                })
            } else {
                json!({
                    "jsonrpc": "2.0",
                    "method": "textDocument/didChange",
                    "params": {
                        "textDocument": { "uri": uri, "version": version },
                        "contentChanges": [{ "text": content }]
                    }
                })
            };
            if write_message(&mut session.stdin, &message).await.is_err() {
                None
            } else {
                match tokio::time::timeout(
                    self.timeout,
                    read_diagnostics(&mut session.stdout, &uri),
                )
                .await
                {
                    Ok(Ok(diagnostics)) => Some(diagnostics),
                    _ => None,
                }
            }
        };

        match outcome {
            Some(diagnostics) => Ok(diagnostics),
            None => {
                *guard = None;
                Ok(Vec::new())
            }
        }
    }
}

async fn read_diagnostics(
    stdout: &mut BufReader<ChildStdout>,
    uri: &str,
) -> std::io::Result<Vec<Diagnostic>> {
    loop {
        let Some(message) = read_message(stdout).await? else {
            return Ok(Vec::new());
        };
        if message.get("method").and_then(Value::as_str) == Some("textDocument/publishDiagnostics")
        {
            let params = &message["params"];
            if params["uri"].as_str() == Some(uri) {
                return Ok(parse_diagnostics(uri, &params["diagnostics"]));
            }
        }
    }
}

async fn write_message(stdin: &mut ChildStdin, value: &Value) -> std::io::Result<()> {
    let body = serde_json::to_vec(value)?;
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    stdin.write_all(header.as_bytes()).await?;
    stdin.write_all(&body).await?;
    stdin.flush().await
}

async fn read_message(stdout: &mut BufReader<ChildStdout>) -> std::io::Result<Option<Value>> {
    let mut length = None;
    loop {
        let mut line = String::new();
        let read = stdout.read_line(&mut line).await?;
        if read == 0 {
            return Ok(None);
        }
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            break;
        }
        if let Some(value) = trimmed.strip_prefix("Content-Length:") {
            length = value.trim().parse::<usize>().ok();
        }
    }
    let length = length.ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, "missing Content-Length")
    })?;
    let mut buffer = vec![0u8; length];
    stdout.read_exact(&mut buffer).await?;
    let value = serde_json::from_slice(&buffer)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    Ok(Some(value))
}

fn parse_diagnostics(uri: &str, diagnostics: &Value) -> Vec<Diagnostic> {
    let path = uri.strip_prefix("file://").unwrap_or(uri).to_string();
    diagnostics
        .as_array()
        .map(|items| {
            items
                .iter()
                .map(|item| Diagnostic {
                    path: path.clone(),
                    line: item["range"]["start"]["line"].as_u64().unwrap_or(0) as u32,
                    column: item["range"]["start"]["character"].as_u64().unwrap_or(0) as u32,
                    severity: severity(item["severity"].as_u64()),
                    message: item["message"].as_str().unwrap_or("").to_string(),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn severity(value: Option<u64>) -> String {
    match value {
        Some(1) => "error",
        Some(2) => "warning",
        Some(3) => "info",
        Some(4) => "hint",
        _ => "diagnostic",
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn missing_server_is_a_no_op() {
        let dir = tempfile::tempdir().expect("tempdir");
        let lsp =
            StdioLsp::new(dir.path(), "/nonexistent/lsp").with_timeout(Duration::from_millis(200));
        assert!(lsp
            .diagnostics("src/main.rs", "fn main() {}")
            .await
            .expect("diagnostics")
            .is_empty());
    }

    #[tokio::test]
    async fn a_silent_server_times_out() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut lsp = StdioLsp::new(dir.path(), "sh").with_timeout(Duration::from_millis(200));
        lsp.command = vec!["sh".into(), "-c".into(), "sleep 10".into()];
        assert!(lsp
            .diagnostics("src/main.rs", "fn main() {}")
            .await
            .expect("diagnostics")
            .is_empty());
    }
}
