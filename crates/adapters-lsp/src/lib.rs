//! Minimal stdio LSP client for post-edit diagnostics.

use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

use async_trait::async_trait;
use orexis_ports::{Diagnostic, Lsp, PortResult};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{ChildStdin, ChildStdout, Command};

pub struct StdioLsp {
    root: PathBuf,
    command: Vec<String>,
    timeout: Duration,
}

impl StdioLsp {
    pub fn new(root: impl Into<PathBuf>, command: impl Into<String>) -> Self {
        Self {
            root: root.into(),
            command: vec![command.into()],
            timeout: Duration::from_secs(5),
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    async fn collect(&self, path: &str, content: &str) -> PortResult<Vec<Diagnostic>> {
        let mut command = Command::new(&self.command[0]);
        command
            .args(&self.command[1..])
            .current_dir(&self.root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(_) => return Ok(Vec::new()),
        };
        let Some(mut stdin) = child.stdin.take() else {
            return Ok(Vec::new());
        };
        let Some(stdout) = child.stdout.take() else {
            return Ok(Vec::new());
        };
        let mut stdout = BufReader::new(stdout);
        let uri = format!("file://{}", self.root.join(path).display());

        let result = tokio::time::timeout(
            self.timeout,
            self.session(&mut stdin, &mut stdout, &uri, content),
        )
        .await;
        let _ = child.kill().await;
        match result {
            Ok(Ok(diagnostics)) => Ok(diagnostics),
            Ok(Err(_)) | Err(_) => Ok(Vec::new()),
        }
    }

    async fn session(
        &self,
        stdin: &mut ChildStdin,
        stdout: &mut BufReader<ChildStdout>,
        uri: &str,
        content: &str,
    ) -> std::io::Result<Vec<Diagnostic>> {
        write_message(
            stdin,
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
            let Some(message) = read_message(stdout).await? else {
                return Ok(Vec::new());
            };
            if message.get("id").and_then(Value::as_u64) == Some(1) {
                break;
            }
        }
        write_message(
            stdin,
            &json!({ "jsonrpc": "2.0", "method": "initialized", "params": {} }),
        )
        .await?;
        write_message(
            stdin,
            &json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": {
                    "textDocument": {
                        "uri": uri,
                        "languageId": "rust",
                        "version": 1,
                        "text": content
                    }
                }
            }),
        )
        .await?;

        loop {
            let Some(message) = read_message(stdout).await? else {
                return Ok(Vec::new());
            };
            if message.get("method").and_then(Value::as_str)
                == Some("textDocument/publishDiagnostics")
            {
                let params = &message["params"];
                if params["uri"].as_str() == Some(uri) {
                    return Ok(parse_diagnostics(uri, &params["diagnostics"]));
                }
            }
        }
    }
}

#[async_trait]
impl Lsp for StdioLsp {
    async fn diagnostics(&self, path: &str, content: &str) -> PortResult<Vec<Diagnostic>> {
        self.collect(path, content).await
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
