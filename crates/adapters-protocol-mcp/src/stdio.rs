//! The stdio shell: owns the server process and drives the sans-io core.
//!
//! One round trip at a time. While waiting for a correlated response, a
//! server-initiated request is refused immediately (-32601) so the wire
//! keeps flowing; transport failures and timeouts are errors, never hangs.
//! Server stderr is captured into a capped tail buffer, never interleaved
//! with the wire.

use std::collections::BTreeMap;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use orexis_adapters_jsonrpc::encode_line;
use orexis_core::ToolSpec;
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

use crate::core::{Event, McpCore};
use crate::error::McpError;
use crate::types::{ClientInfo, McpPrompt, McpResource, ResourceContent, ToolCallResult};

const STDERR_CAP: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq)]
pub struct McpServerConfig {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
}

#[derive(Debug)]
pub struct McpStdioClient {
    core: McpCore,
    /// Kept alive for RAII; dropping it kills the server.
    #[allow(dead_code)]
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    stderr: Arc<Mutex<String>>,
    timeout: Duration,
}

impl McpStdioClient {
    pub async fn spawn(config: &McpServerConfig, timeout: Duration) -> Result<Self, McpError> {
        let mut command = Command::new(&config.command);
        command
            .args(&config.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        for (key, value) in &config.env {
            command.env(key, value);
        }
        let mut child = command
            .spawn()
            .map_err(|error| McpError::Transport(error.to_string()))?;
        let Some(stdin) = child.stdin.take() else {
            return Err(McpError::Transport("missing server stdin".into()));
        };
        let Some(stdout) = child.stdout.take() else {
            return Err(McpError::Transport("missing server stdout".into()));
        };
        let Some(stderr) = child.stderr.take() else {
            return Err(McpError::Transport("missing server stderr".into()));
        };
        let captured = Arc::new(Mutex::new(String::new()));
        tokio::spawn(capture_stderr(stderr, captured.clone()));

        let mut client = Self {
            core: McpCore::new(ClientInfo {
                name: "orexis".into(),
                version: env!("CARGO_PKG_VERSION").into(),
            }),
            child,
            stdin,
            stdout: BufReader::new(stdout),
            stderr: captured,
            timeout,
        };
        client.core.initialize()?;
        client
            .until(|event| match event {
                Event::Initialized(_) => Some(()),
                _ => None,
            })
            .await?;
        Ok(client)
    }

    pub async fn list_tools(
        &mut self,
        cursor: Option<String>,
    ) -> Result<(Vec<ToolSpec>, Option<String>), McpError> {
        self.core.list_tools(cursor)?;
        self.until(|event| match event {
            Event::ToolsListed { tools, next_cursor } => Some((tools.clone(), next_cursor.clone())),
            _ => None,
        })
        .await
    }

    pub async fn call_tool(
        &mut self,
        name: &str,
        arguments: Option<Value>,
    ) -> Result<ToolCallResult, McpError> {
        self.core.call_tool(name, arguments)?;
        self.until(|event| match event {
            Event::ToolCalled(result) => Some(result.clone()),
            _ => None,
        })
        .await
    }

    pub async fn list_resources(
        &mut self,
        cursor: Option<String>,
    ) -> Result<(Vec<McpResource>, Option<String>), McpError> {
        self.core.list_resources(cursor)?;
        self.until(|event| match event {
            Event::ResourcesListed {
                resources,
                next_cursor,
            } => Some((resources.clone(), next_cursor.clone())),
            _ => None,
        })
        .await
    }

    pub async fn read_resource(&mut self, uri: &str) -> Result<Vec<ResourceContent>, McpError> {
        self.core.read_resource(uri)?;
        self.until(|event| match event {
            Event::ResourceRead { contents } => Some(contents.clone()),
            _ => None,
        })
        .await
    }

    pub async fn list_prompts(
        &mut self,
        cursor: Option<String>,
    ) -> Result<(Vec<McpPrompt>, Option<String>), McpError> {
        self.core.list_prompts(cursor)?;
        self.until(|event| match event {
            Event::PromptsListed {
                prompts,
                next_cursor,
            } => Some((prompts.clone(), next_cursor.clone())),
            _ => None,
        })
        .await
    }

    pub async fn get_prompt(
        &mut self,
        name: &str,
        arguments: Option<Value>,
    ) -> Result<Value, McpError> {
        self.core.get_prompt(name, arguments)?;
        self.until(|event| match event {
            Event::PromptGot(value) => Some(value.clone()),
            _ => None,
        })
        .await
    }

    pub async fn ping(&mut self) -> Result<(), McpError> {
        self.core.ping()?;
        self.until(|event| matches!(event, Event::Pong).then_some(()))
            .await
    }

    /// The captured server stderr tail, capped at 64KB.
    pub fn stderr_text(&self) -> String {
        self.stderr.lock().expect("stderr lock").clone()
    }

    async fn flush_outgoing(&mut self) -> Result<(), McpError> {
        while let Some(message) = self.core.pop_outgoing() {
            let line = encode_line(&message);
            self.stdin
                .write_all(line.as_bytes())
                .await
                .map_err(|error| McpError::Transport(error.to_string()))?;
            self.stdin
                .flush()
                .await
                .map_err(|error| McpError::Transport(error.to_string()))?;
        }
        Ok(())
    }

    async fn read_line(&mut self) -> Result<Option<String>, McpError> {
        loop {
            let mut line = String::new();
            let read = self
                .stdout
                .read_line(&mut line)
                .await
                .map_err(|error| McpError::Transport(error.to_string()))?;
            if read == 0 {
                return Ok(None);
            }
            if !line.trim().is_empty() {
                return Ok(Some(line));
            }
        }
    }

    async fn until<E>(&mut self, matches: impl Fn(&Event) -> Option<E>) -> Result<E, McpError> {
        self.flush_outgoing().await?;
        loop {
            let read = tokio::time::timeout(self.timeout, self.read_line())
                .await
                .map_err(|_| McpError::Timeout)?;
            let Some(line) = read? else {
                return Err(McpError::Transport("server closed the stream".into()));
            };
            for event in self.core.receive(&line)? {
                if let Some(value) = matches(&event) {
                    self.flush_outgoing().await?;
                    return Ok(value);
                }
                if let Event::ServerRequest { id, .. } = event {
                    self.core.refuse(id);
                    self.flush_outgoing().await?;
                }
            }
        }
    }
}

async fn capture_stderr(stderr: tokio::process::ChildStderr, into: Arc<Mutex<String>>) {
    let mut stderr = BufReader::new(stderr);
    let mut line = String::new();
    loop {
        line.clear();
        match stderr.read_line(&mut line).await {
            Ok(0) | Err(_) => return,
            Ok(_) => {
                let mut captured = into.lock().expect("stderr lock");
                captured.push_str(&line);
                if captured.len() > STDERR_CAP {
                    let mut excess = captured.len() - STDERR_CAP;
                    while !captured.is_char_boundary(excess) {
                        excess += 1;
                    }
                    let keep = captured[excess..]
                        .find('\n')
                        .map(|offset| excess + offset + 1)
                        .unwrap_or(captured.len());
                    captured.drain(..keep);
                }
            }
        }
    }
}
