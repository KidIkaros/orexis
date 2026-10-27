//! A2A remote agents as part of the tool surface (ADR-0017 §3).
//!
//! A remote agent is reached through the same tool surface as MCP and
//! UTCP: namespaced `a2a/<agent>/<action>` tools, the same permission
//! gate, the same caps. A remote agent's output is third-party content —
//! it is delimited and capped before it reaches a model context, exactly
//! as MCP tool output is.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use orexis_adapters_http::Transport;
use orexis_core::{Observation, ToolSpec};
use orexis_ports::{ExternalTools, PortError, PortResult, ToolExecutor};
use serde_json::Value;

use crate::core::{AgentCard, Outcome};
use crate::http::{A2aClient, A2aClientConfig};

const DESCRIPTION_CAP: usize = 500;
const OUTPUT_CAP: usize = 64 * 1024;
const TIMEOUT: Duration = Duration::from_secs(60);

/// One configured remote agent.
#[derive(Debug, Clone, PartialEq)]
pub struct A2aPeer {
    /// The name this agent is known by; it namespaces the tool surface.
    pub name: String,
    /// The agent's base URL.
    pub base_url: String,
    /// Extra request headers (authorization, and so on).
    pub headers: Vec<(String, String)>,
}

/// The tools one remote agent exposes.
pub struct A2aTools {
    peers: Vec<A2aPeer>,
    transport: Arc<dyn Transport>,
    clients: tokio::sync::Mutex<BTreeMap<String, A2aClient>>,
    cards: tokio::sync::Mutex<BTreeMap<String, AgentCard>>,
    problems: Vec<String>,
}

impl A2aTools {
    pub fn new(peers: Vec<A2aPeer>, transport: Arc<dyn Transport>) -> Self {
        Self {
            peers,
            transport,
            clients: tokio::sync::Mutex::new(BTreeMap::new()),
            cards: tokio::sync::Mutex::new(BTreeMap::new()),
            problems: Vec::new(),
        }
    }

    pub fn configured(&self) -> usize {
        self.peers.len()
    }

    /// Fetch every peer's card; returns per-agent failures.
    pub async fn discover_all(&self) -> Vec<(String, String)> {
        let mut failures = Vec::new();
        for peer in &self.peers {
            match self.discover(peer).await {
                Ok(_) => {}
                Err(error) => failures.push((peer.name.clone(), error.to_string())),
            }
        }
        failures
    }

    /// The card of one agent, once it has been discovered.
    async fn discover(&self, peer: &A2aPeer) -> Result<AgentCard, String> {
        let mut config = A2aClientConfig::new(&peer.name, &peer.base_url);
        for (name, value) in &peer.headers {
            config = config.with_header(name, value);
        }
        let mut client = A2aClient::new(config, self.transport.clone(), TIMEOUT);
        let card = client.discover().await.map_err(|error| error.to_string())?;
        let mut clients = self.clients.lock().await;
        clients.insert(peer.name.clone(), client);
        let mut cards = self.cards.lock().await;
        cards.insert(peer.name.clone(), card.clone());
        Ok(card)
    }

    /// The namespaced specs for every discovered agent.
    pub async fn specs(&self) -> Vec<ToolSpec> {
        let cards = self.cards.lock().await;
        let mut specs = Vec::new();
        for card in cards.values() {
            let description = if card.description.is_empty() {
                format!("the remote agent {}", card.name)
            } else {
                card.description.clone()
            };
            specs.push(ToolSpec {
                name: format!("a2a/{}/send", card.name),
                description: cap_description(&description),
                schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "text": {"type": "string", "description": "the message to send"},
                    },
                    "required": ["text"],
                }),
            });
            specs.push(ToolSpec {
                name: format!("a2a/{}/task", card.name),
                description: cap_description(&format!(
                    "read the status of a task on the remote agent {}",
                    card.name
                )),
                schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "id": {"type": "string", "description": "the task id"},
                    },
                    "required": ["id"],
                }),
            });
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

/// The delimiter marks remote output as data; a hostile agent must not be
/// able to close it early, so closing tags inside the output are escaped.
fn delimit(agent: &str, action: &str, output: &str) -> String {
    let escaped = output.replace("</a2a_result", "<\\/a2a_result");
    format!("<a2a_result agent=\"{agent}\" action=\"{action}\">\n{escaped}\n</a2a_result>")
}

/// Load `*.json` peer configs from each directory in order; later
/// directories win by agent name. Missing directories are normal;
/// unreadable or malformed files are reported as problems.
pub fn load_peer_configs(dirs: &[std::path::PathBuf]) -> (Vec<A2aPeer>, Vec<String>) {
    let mut by_name = BTreeMap::new();
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
                Ok(value) => match parse_peer_file(&value) {
                    Ok(peers) => {
                        for peer in peers {
                            by_name.insert(peer.name.clone(), peer);
                        }
                    }
                    Err(error) => problems.push(format!("{}: {error}", path.display())),
                },
            }
        }
    }
    (by_name.into_values().collect(), problems)
}

/// The peers of one config file: `{"peers": [...]}`.
fn parse_peer_file(value: &Value) -> Result<Vec<A2aPeer>, String> {
    let peers = value
        .get("peers")
        .and_then(Value::as_array)
        .ok_or_else(|| "missing a peers array".to_string())?;
    peers
        .iter()
        .map(|peer| {
            let name = peer
                .get("name")
                .and_then(Value::as_str)
                .ok_or_else(|| "peer without a name".to_string())?;
            if name.contains('/') {
                return Err(format!("peer name {name:?} contains '/'"));
            }
            let base_url = peer
                .get("base_url")
                .or_else(|| peer.get("url"))
                .and_then(Value::as_str)
                .ok_or_else(|| format!("peer {name:?} without a base_url"))?;
            Ok(A2aPeer {
                name: name.to_string(),
                base_url: base_url.to_string(),
                headers: peer
                    .get("headers")
                    .and_then(Value::as_object)
                    .map(|headers| {
                        headers
                            .iter()
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

fn cap_output(output: &str) -> String {
    if output.chars().count() > OUTPUT_CAP {
        let mut capped = output.chars().take(OUTPUT_CAP).collect::<String>();
        capped.push_str("\n[truncated at 64KB]");
        capped
    } else {
        output.to_string()
    }
}

/// The rendered form of one outcome.
fn render(outcome: &Outcome) -> String {
    match outcome {
        Outcome::Message { text } => text.clone(),
        Outcome::Task(task) => {
            let mut rendered = format!("task {} is {}", task.id, task.raw_state);
            if !task.text.is_empty() {
                rendered.push('\n');
                rendered.push_str(&task.text);
            }
            rendered
        }
    }
}

/// Split `a2a/<agent>/<action>` into its parts.
fn parse_name(name: &str) -> Option<(String, String)> {
    let rest = name.strip_prefix("a2a/")?;
    let (agent, action) = rest.split_once('/')?;
    if agent.is_empty() || action.is_empty() {
        return None;
    }
    Some((agent.to_string(), action.to_string()))
}

#[async_trait]
impl ExternalTools for A2aTools {
    fn kind(&self) -> &'static str {
        "a2a"
    }

    async fn connect_all(&self) -> Vec<(String, String)> {
        let mut failures: Vec<(String, String)> = self
            .problems
            .iter()
            .map(|problem| ("manual".to_string(), problem.clone()))
            .collect();
        failures.extend(self.discover_all().await);
        failures
    }

    async fn specs(&self) -> Vec<ToolSpec> {
        A2aTools::specs(self).await
    }
}

#[async_trait]
impl ToolExecutor for A2aTools {
    async fn execute(&self, name: &str, args: &str) -> PortResult<Observation> {
        let (agent, action) =
            parse_name(name).ok_or_else(|| PortError(format!("not an a2a tool name: {name}")))?;
        let arguments: Value = if args.trim().is_empty() {
            Value::Object(Default::default())
        } else {
            serde_json::from_str(args)
                .map_err(|error| PortError(format!("invalid tool arguments: {error}")))?
        };
        let mut clients = self.clients.lock().await;
        let Some(client) = clients.get_mut(&agent) else {
            return Err(PortError(format!("a2a agent is not connected: {agent}")));
        };
        let outcome = match action.as_str() {
            "send" => {
                let text = arguments
                    .get("text")
                    .and_then(Value::as_str)
                    .ok_or_else(|| PortError("the send action needs a text argument".into()))?;
                client
                    .send_message(text)
                    .await
                    .map_err(|error| PortError(error.to_string()))?
            }
            "task" => {
                let id = arguments
                    .get("id")
                    .and_then(Value::as_str)
                    .ok_or_else(|| PortError("the task action needs an id argument".into()))?;
                client
                    .get_task(id)
                    .await
                    .map_err(|error| PortError(error.to_string()))?
            }
            other => {
                return Err(PortError(format!("unknown a2a action: {other}")));
            }
        };
        Ok(Observation {
            name: name.to_string(),
            output: delimit(&agent, &action, &cap_output(&render(&outcome))),
        })
    }
}
