//! MCP tools as part of the tool surface (ADR-0017 §3).
//!
//! `McpTools` owns the configured servers, connects them, exposes their
//! tools as namespaced `mcp/<server>/<tool>` specs, and executes calls.
//! Server output is untrusted content: it is delimited, capped, and its
//! descriptions are capped before they reach a prompt.

use std::collections::BTreeMap;

use async_trait::async_trait;
use orexis_core::{Observation, ToolSpec};
use orexis_ports::{ExternalTools, PortError, PortResult, ToolExecutor};
use serde_json::Value;
use tokio::sync::Mutex;

use crate::stdio::{McpServerConfig, McpStdioClient};
use crate::types::ToolContent;

const DESCRIPTION_CAP: usize = 500;
const OUTPUT_CAP: usize = 64 * 1024;
const MAX_PAGES: usize = 10;

pub struct McpTools {
    configs: Vec<McpServerConfig>,
    clients: Mutex<Vec<(String, McpStdioClient)>>,
}

impl McpTools {
    pub fn new(configs: Vec<McpServerConfig>) -> Self {
        Self {
            configs,
            clients: Mutex::new(Vec::new()),
        }
    }

    pub fn configured(&self) -> usize {
        self.configs.len()
    }

    /// Connect every configured server once; returns per-server failures.
    /// Connected servers stay connected for the client's lifetime.
    pub async fn connect_all(&self) -> Vec<(String, String)> {
        let mut clients = self.clients.lock().await;
        let mut failures = Vec::new();
        for config in &self.configs {
            if clients.iter().any(|(name, _)| name == &config.name) {
                continue;
            }
            match McpStdioClient::spawn(config, std::time::Duration::from_secs(10)).await {
                Ok(client) => clients.push((config.name.clone(), client)),
                Err(error) => failures.push((config.name.clone(), error.to_string())),
            }
        }
        failures
    }

    /// Namespaced specs for every connected server, following pagination.
    pub async fn specs(&self) -> Vec<ToolSpec> {
        let mut clients = self.clients.lock().await;
        let mut specs = Vec::new();
        for (server, client) in clients.iter_mut() {
            let mut cursor = None;
            for _ in 0..MAX_PAGES {
                let Ok((tools, next)) = client.list_tools(cursor.take()).await else {
                    break;
                };
                for tool in tools {
                    specs.push(ToolSpec {
                        name: format!("mcp/{server}/{}", tool.name),
                        description: cap_description(&tool.description),
                        schema: tool.schema,
                    });
                }
                match next {
                    Some(next) => cursor = Some(next),
                    None => break,
                }
            }
        }
        specs
    }
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

/// The delimiter marks server output as data; a hostile server must not be
/// able to close it early, so closing tags inside the output are escaped.
fn delimit(server: &str, tool: &str, output: &str) -> String {
    let escaped = output.replace("</mcp_result", "<\\/mcp_result");
    format!("<mcp_result server=\"{server}\" tool=\"{tool}\">\n{escaped}\n</mcp_result>")
}

fn render_content(content: &[ToolContent]) -> String {
    content
        .iter()
        .map(|block| match block {
            ToolContent::Text { text } => text.clone(),
            ToolContent::Image { mime_type, .. } => format!("[image block: {mime_type}]"),
            ToolContent::Resource { resource } => {
                let uri = resource.get("uri").and_then(Value::as_str).unwrap_or("");
                format!("[resource block: {uri}]")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn parse_name(name: &str) -> Option<(String, String)> {
    let rest = name.strip_prefix("mcp/")?;
    let (server, tool) = rest.split_once('/')?;
    if server.is_empty() || tool.is_empty() {
        return None;
    }
    Some((server.to_string(), tool.to_string()))
}

#[async_trait]
impl ExternalTools for McpTools {
    async fn connect_all(&self) -> Vec<(String, String)> {
        McpTools::connect_all(self).await
    }

    async fn specs(&self) -> Vec<ToolSpec> {
        McpTools::specs(self).await
    }
}

#[async_trait]
impl ToolExecutor for McpTools {
    async fn execute(&self, name: &str, args: &str) -> PortResult<Observation> {
        let (server, tool) =
            parse_name(name).ok_or_else(|| PortError(format!("not an mcp tool name: {name}")))?;
        let arguments = if args.trim().is_empty() {
            None
        } else {
            let value: Value = serde_json::from_str(args)
                .map_err(|error| PortError(format!("invalid tool arguments: {error}")))?;
            Some(value)
        };
        let mut clients = self.clients.lock().await;
        let Some((_, client)) = clients.iter_mut().find(|(known, _)| known == &server) else {
            return Err(PortError(format!("mcp server is not connected: {server}")));
        };
        let result = client
            .call_tool(&tool, arguments)
            .await
            .map_err(|error| PortError(error.to_string()))?;
        let mut output = render_content(&result.content);
        if output.chars().count() > OUTPUT_CAP {
            output = output.chars().take(OUTPUT_CAP).collect();
            output.push_str("\n[truncated at 64KB]");
        }
        if result.is_error {
            output = format!("[tool error] {output}");
        }
        Ok(Observation {
            name: name.to_string(),
            output: delimit(&server, &tool, &output),
        })
    }
}

/// Load `*.json` server configs from each directory in order; later
/// directories win by server name. Missing directories are normal; unreadable
/// or malformed files are reported as problems.
pub fn load_server_configs(dirs: &[std::path::PathBuf]) -> (Vec<McpServerConfig>, Vec<String>) {
    let mut by_name: BTreeMap<String, McpServerConfig> = BTreeMap::new();
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
                    serde_json::from_str::<Value>(&text).map_err(|error| error.to_string())
                }) {
                Err(error) => problems.push(format!("{}: {error}", path.display())),
                Ok(value) => match parse_server_file(&value) {
                    Ok(servers) => {
                        for config in servers {
                            by_name.insert(config.name.clone(), config);
                        }
                    }
                    Err(error) => problems.push(format!("{}: {error}", path.display())),
                },
            }
        }
    }
    (by_name.into_values().collect(), problems)
}

fn parse_server_file(value: &Value) -> Result<Vec<McpServerConfig>, String> {
    let servers = value
        .get("servers")
        .and_then(Value::as_array)
        .ok_or_else(|| "missing a servers array".to_string())?;
    servers
        .iter()
        .map(|server| {
            let name = server
                .get("name")
                .and_then(Value::as_str)
                .ok_or_else(|| "server without a name".to_string())?;
            if name.contains('/') {
                return Err(format!("server name {name:?} contains '/'"));
            }
            let command = server
                .get("command")
                .and_then(Value::as_str)
                .ok_or_else(|| format!("server {name:?} without a command"))?;
            Ok(McpServerConfig {
                name: name.to_string(),
                command: command.to_string(),
                args: server
                    .get("args")
                    .and_then(Value::as_array)
                    .map(|args| {
                        args.iter()
                            .filter_map(Value::as_str)
                            .map(str::to_string)
                            .collect()
                    })
                    .unwrap_or_default(),
                env: server
                    .get("env")
                    .and_then(Value::as_object)
                    .map(|env| {
                        env.iter()
                            .filter_map(|(key, value)| {
                                value.as_str().map(|value| (key.clone(), value.to_string()))
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests;
