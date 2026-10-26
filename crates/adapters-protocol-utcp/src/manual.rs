//! Provider manuals: a tool source and how to reach it.
//!
//! Serialized form matches Kindred's UTCP `providers.json` convention:
//! either a JSON array of providers or an object with a `providers` array.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum UtcpError {
    #[error("providers parse failed: {0}")]
    Parse(String),
    #[error("providers config must be an array or {{providers: []}}")]
    Shape,
    #[error("provider {0:?}: {1}")]
    Provider(String, String),
    #[error("tool {0:?} not found in the index")]
    UnknownTool(String),
    #[error("provider {0:?} is not registered")]
    UnknownProvider(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderType {
    /// A CLI program invoked as a subprocess.
    Cli,
    /// An HTTP/REST endpoint.
    Http,
    /// An MCP server (stdio). MCP is a transport inside UTCP.
    Mcp,
}

impl ProviderType {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Cli => "cli",
            Self::Http => "http",
            Self::Mcp => "mcp",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Provider {
    pub name: String,
    pub provider_type: ProviderType,
    /// Endpoint: URL for http, command path for cli, server command for mcp.
    #[serde(default)]
    pub endpoint: String,
    /// For CLI providers: the base command (split on whitespace).
    #[serde(default)]
    pub command: Vec<String>,
    /// For MCP providers: the server command + args.
    #[serde(default, rename = "command_args")]
    pub command_args: Vec<String>,
    /// Optional auth token (sent as a Bearer token for HTTP).
    #[serde(default)]
    pub auth_token: Option<String>,
    #[serde(default)]
    pub description: String,
}

impl Provider {
    /// Structural validation: a name, and whatever its transport needs.
    pub fn validate(&self) -> Result<(), UtcpError> {
        if self.name.is_empty() || self.name.contains('/') {
            return Err(UtcpError::Provider(
                self.name.clone(),
                "name must be non-empty and free of '/'".into(),
            ));
        }
        let reachable = match self.provider_type {
            ProviderType::Cli => !self.command.is_empty() || !self.endpoint.is_empty(),
            ProviderType::Http => !self.endpoint.is_empty(),
            ProviderType::Mcp => !self.command_args.is_empty() || !self.endpoint.is_empty(),
        };
        if !reachable {
            return Err(UtcpError::Provider(
                self.name.clone(),
                format!(
                    "{} provider has no endpoint or command",
                    self.provider_type.name()
                ),
            ));
        }
        Ok(())
    }

    /// The MCP server command for an Mcp provider.
    pub(crate) fn mcp_command(&self) -> Vec<String> {
        if self.command_args.is_empty() {
            vec![self.endpoint.clone()]
        } else {
            self.command_args.clone()
        }
    }
}

/// Parse a providers manual: an array of providers or `{providers: []}`.
pub fn parse_providers(json: &str) -> Result<Vec<Provider>, UtcpError> {
    let value: serde_json::Value =
        serde_json::from_str(json).map_err(|error| UtcpError::Parse(error.to_string()))?;
    let entries = if value.is_array() {
        value
    } else if let Some(array) = value.get("providers").and_then(serde_json::Value::as_array) {
        serde_json::Value::Array(array.clone())
    } else {
        return Err(UtcpError::Shape);
    };
    let providers: Vec<Provider> = serde_json::from_value(entries)
        .map_err(|error| UtcpError::Parse(format!("provider list: {error}")))?;
    for provider in &providers {
        provider.validate()?;
    }
    Ok(providers)
}

#[cfg(test)]
mod tests;
