//! UTCP transports: the I/O shells the sans-io core instructs (M7-09).
//!
//! `UtcpTools` owns the registered providers and executes their
//! discovery/call instructions: HTTP over `orexis-adapters-http`, CLI
//! through the sandboxed `CommandRunner` port (never a raw spawn), MCP
//! through the `PeerFactory` port (composition injects the client, so
//! protocol adapters never depend on each other).
//!
//! Implements `ExternalTools`: providers enter the same tool surface as
//! MCP servers — namespaced `utcp/<provider>/<tool>`, same gate,
//! delimiting, and caps.

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use orexis_adapters_http::Transport;
use orexis_core::{Observation, ToolSpec};
use orexis_ports::{
    CommandOutput, CommandRunner, ExternalTools, PeerFactory, PeerSession, PortError, PortResult,
    ToolExecutor,
};
use serde_json::Value;
use tokio::sync::Mutex;

use crate::core::{CallInstruction, DiscoverInstruction, ToolSchema, UtcpCore};
use crate::manual::Provider;

const DESCRIPTION_CAP: usize = 500;
const OUTPUT_CAP: usize = 64 * 1024;
const COMMAND_TIMEOUT: Duration = Duration::from_secs(60);

/// A UTCP client: provider manuals in, namespaced tools out.
pub struct UtcpTools {
    http: Arc<dyn Transport>,
    commands: Arc<dyn CommandRunner>,
    peers: Arc<dyn PeerFactory>,
    core: Mutex<UtcpCore>,
    sessions: Mutex<HashMap<String, Box<dyn PeerSession>>>,
    problems: Vec<String>,
}

impl UtcpTools {
    pub fn new(
        providers: Vec<Provider>,
        http: Arc<dyn Transport>,
        commands: Arc<dyn CommandRunner>,
        peers: Arc<dyn PeerFactory>,
    ) -> Self {
        let mut core = UtcpCore::new();
        let mut problems = Vec::new();
        for provider in providers {
            if let Err(error) = core.register(provider) {
                problems.push(error.to_string());
            }
        }
        Self {
            http,
            commands,
            peers,
            core: Mutex::new(core),
            sessions: Mutex::new(HashMap::new()),
            problems,
        }
    }

    async fn discover(&self, instruction: &DiscoverInstruction) -> Result<Vec<ToolSchema>, String> {
        match instruction {
            DiscoverInstruction::Http { url, bearer } => {
                let body = self
                    .http
                    .get(url, auth_headers(bearer))
                    .await
                    .map_err(|error| error.to_string())?;
                parse_tool_list(&body)
            }
            DiscoverInstruction::Cli { program, args } => {
                let output = self
                    .commands
                    .run(program, args, COMMAND_TIMEOUT)
                    .await
                    .map_err(|error| error.to_string())?;
                parse_tool_list(&output.stdout)
            }
            DiscoverInstruction::Mcp { command } => {
                let mut sessions = self.sessions.lock().await;
                let peer = self.peer(&mut sessions, command).await?;
                let specs = peer.tools().await.map_err(|error| error.to_string())?;
                Ok(specs
                    .into_iter()
                    .map(|spec| ToolSchema {
                        name: spec.name,
                        description: spec.description,
                        input_schema: spec.schema,
                        provider: String::new(),
                    })
                    .collect())
            }
        }
    }

    /// The cached peer for a command argv, connecting on first use. The
    /// sessions lock stays held by the caller so calls serialize per peer.
    async fn peer<'a>(
        &self,
        sessions: &'a mut HashMap<String, Box<dyn PeerSession>>,
        command: &[String],
    ) -> Result<&'a mut dyn PeerSession, String> {
        let key = command.join("\u{1}");
        if !sessions.contains_key(&key) {
            if command.is_empty() {
                return Err("mcp provider has an empty command".to_string());
            }
            let session = self
                .peers
                .connect(command)
                .await
                .map_err(|error| error.to_string())?;
            sessions.insert(key.clone(), session);
        }
        sessions
            .get_mut(&key)
            .map(|peer| &mut **peer as &mut dyn PeerSession)
            .ok_or_else(|| "peer session missing".to_string())
    }
}

fn auth_headers(bearer: &Option<String>) -> Vec<(String, String)> {
    bearer
        .iter()
        .map(|token| ("authorization".to_string(), format!("Bearer {token}")))
        .collect()
}

/// A tool list is a JSON array of schemas or `{tools: [...]}` — tolerant
/// like the provider manuals themselves.
fn parse_tool_list(body: &str) -> Result<Vec<ToolSchema>, String> {
    let value: Value =
        serde_json::from_str(body).map_err(|error| format!("tool list parse: {error}"))?;
    let entries = if value.is_array() {
        value
    } else if let Some(array) = value.get("tools").and_then(Value::as_array) {
        Value::Array(array.clone())
    } else {
        return Err("tool list must be an array or {tools: []}".to_string());
    };
    serde_json::from_value(entries).map_err(|error| format!("tool list parse: {error}"))
}

fn cap_description(description: &str) -> String {
    let mut description = description
        .chars()
        .take(DESCRIPTION_CAP)
        .collect::<String>();
    if description.chars().count() == DESCRIPTION_CAP {
        description.push('…');
    }
    description
}

/// The delimiter marks provider output as data; a hostile provider must
/// not close it early, so closing tags inside the output are escaped.
fn delimit(provider: &str, tool: &str, output: &str) -> String {
    let escaped = output.replace("</utcp_result", "<\\/utcp_result");
    format!("<utcp_result provider=\"{provider}\" tool=\"{tool}\">\n{escaped}\n</utcp_result>")
}

fn cap_output(output: &str) -> String {
    if output.chars().count() > OUTPUT_CAP {
        let mut capped = output.chars().take(OUTPUT_CAP).collect::<String>();
        capped.push_str("\n[truncated at 64KB]");
        capped
    } else {
        output.to_string()
    }
}

/// CLI call results: `{success, content}` when the program answers
/// structured JSON, else raw stdout (stderr when stdout is empty) keyed
/// on the exit status — Kindred's wire contract.
fn render_cli(output: &CommandOutput) -> String {
    if let Ok(value) = serde_json::from_str::<Value>(&output.stdout) {
        if let (Some(success), Some(content)) = (
            value.get("success").and_then(Value::as_bool),
            value.get("content"),
        ) {
            let content = content
                .as_str()
                .map(str::to_string)
                .unwrap_or_else(|| content.to_string());
            return if success {
                content
            } else {
                format!("[tool error] {content}")
            };
        }
    }
    let body = if output.stdout.trim().is_empty() {
        output.stderr.clone()
    } else {
        output.stdout.clone()
    };
    if output.timed_out {
        format!("[tool error] timed out\n{body}")
    } else if output.code == Some(0) {
        body
    } else {
        let status = output
            .code
            .map(|code| format!("exit {code}"))
            .unwrap_or_else(|| "signal".to_string());
        format!("[tool error] {status}\n{body}")
    }
}

fn parse_name(name: &str) -> Option<(String, String)> {
    let rest = name.strip_prefix("utcp/")?;
    let (provider, tool) = rest.split_once('/')?;
    if provider.is_empty() || tool.is_empty() {
        return None;
    }
    Some((provider.to_string(), tool.to_string()))
}

#[async_trait]
impl ExternalTools for UtcpTools {
    fn kind(&self) -> &'static str {
        "utcp"
    }

    /// Discover every registered provider's tools; per-provider failures
    /// are reported, not fatal.
    async fn connect_all(&self) -> Vec<(String, String)> {
        let mut failures: Vec<(String, String)> = self
            .problems
            .iter()
            .map(|problem| ("manual".to_string(), problem.clone()))
            .collect();
        let instructions = self.core.lock().await.discover_instructions();
        for (provider, instruction) in instructions {
            match self.discover(&instruction).await {
                Ok(tools) => self.core.lock().await.record_discovery(&provider, tools),
                Err(error) => failures.push((provider, error)),
            }
        }
        failures
    }

    async fn specs(&self) -> Vec<ToolSpec> {
        self.core
            .lock()
            .await
            .tools()
            .iter()
            .map(|tool| ToolSpec {
                name: format!("utcp/{}/{}", tool.provider, tool.name),
                description: cap_description(&tool.description),
                schema: tool.input_schema.clone(),
            })
            .collect()
    }
}

#[async_trait]
impl ToolExecutor for UtcpTools {
    async fn execute(&self, name: &str, args: &str) -> PortResult<Observation> {
        let (provider, tool) =
            parse_name(name).ok_or_else(|| PortError(format!("not a utcp tool name: {name}")))?;
        let arguments = if args.trim().is_empty() {
            Value::Object(Default::default())
        } else {
            serde_json::from_str(args)
                .map_err(|error| PortError(format!("invalid tool arguments: {error}")))?
        };
        let instruction = self
            .core
            .lock()
            .await
            .call_instruction(&provider, &tool, arguments)
            .map_err(|error| PortError(error.to_string()))?;
        let output = match instruction {
            CallInstruction::Http { url, bearer, body } => self
                .http
                .post(&url, auth_headers(&bearer), body.to_string())
                .await
                .map_err(|error| PortError(error.to_string()))?,
            CallInstruction::Cli { program, args } => {
                let output = self
                    .commands
                    .run(&program, &args, COMMAND_TIMEOUT)
                    .await
                    .map_err(|error| PortError(error.to_string()))?;
                render_cli(&output)
            }
            CallInstruction::Mcp {
                command,
                tool,
                args,
            } => {
                let mut sessions = self.sessions.lock().await;
                let peer = self
                    .peer(&mut sessions, &command)
                    .await
                    .map_err(PortError)?;
                let result = peer.call(&tool, &args.to_string()).await?;
                if result.is_error {
                    format!("[tool error] {}", result.text)
                } else {
                    result.text
                }
            }
        };
        Ok(Observation {
            name: name.to_string(),
            output: delimit(&provider, &tool, &cap_output(&output)),
        })
    }
}

/// Load `*.json` provider manuals from each directory in order; later
/// directories win by provider name. Missing directories are normal;
/// unreadable or malformed files are reported as problems.
pub fn load_manuals(dirs: &[PathBuf]) -> (Vec<Provider>, Vec<String>) {
    let mut by_name: BTreeMap<String, Provider> = BTreeMap::new();
    let mut problems = Vec::new();
    for dir in dirs {
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }
            match std::fs::read_to_string(&path)
                .map_err(|error| error.to_string())
                .and_then(|text| {
                    crate::manual::parse_providers(&text).map_err(|error| error.to_string())
                }) {
                Err(error) => problems.push(format!("{}: {error}", path.display())),
                Ok(providers) => {
                    for provider in providers {
                        by_name.insert(provider.name.clone(), provider);
                    }
                }
            }
        }
    }
    (by_name.into_values().collect(), problems)
}

#[cfg(test)]
mod tests;
