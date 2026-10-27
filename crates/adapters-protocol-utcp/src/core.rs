//! The tool index and the call/discovery instructions.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::manual::{Provider, ProviderType, UtcpError};

/// An indexed tool schema — discovered from a provider and cached locally.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolSchema {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default, rename = "inputSchema")]
    pub input_schema: Value,
    #[serde(default)]
    pub provider: String,
}

/// What a transport shell must do to discover a provider's tools.
#[derive(Debug, Clone, PartialEq)]
pub enum DiscoverInstruction {
    Http { url: String, bearer: Option<String> },
    Cli { program: String, args: Vec<String> },
    Mcp { command: Vec<String> },
}

/// What a transport shell must do to call one tool.
#[derive(Debug, Clone, PartialEq)]
pub enum CallInstruction {
    Http {
        url: String,
        bearer: Option<String>,
        body: Value,
    },
    Cli {
        program: String,
        args: Vec<String>,
    },
    Mcp {
        command: Vec<String>,
        tool: String,
        args: Value,
    },
}

/// The universal client core: manuals in, instructions out, index local.
#[derive(Default)]
pub struct UtcpCore {
    providers: Vec<Provider>,
    index: Vec<ToolSchema>,
}

impl UtcpCore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register one already-parsed provider (validated again here).
    pub fn register(&mut self, provider: Provider) -> Result<(), UtcpError> {
        provider.validate()?;
        self.providers.push(provider);
        Ok(())
    }

    /// Parse a providers manual and register its providers (appending).
    pub fn load_providers(&mut self, json: &str) -> Result<usize, UtcpError> {
        let providers = crate::manual::parse_providers(json)?;
        let count = providers.len();
        self.providers.extend(providers);
        Ok(count)
    }

    pub fn provider_count(&self) -> usize {
        self.providers.len()
    }

    pub fn provider_names(&self) -> Vec<String> {
        let mut names = self
            .providers
            .iter()
            .map(|provider| provider.name.clone())
            .collect::<Vec<_>>();
        names.sort();
        names
    }

    fn provider(&self, name: &str) -> Option<&Provider> {
        self.providers.iter().find(|provider| provider.name == name)
    }

    fn bearer(provider: &Provider) -> Option<String> {
        provider
            .auth_token
            .clone()
            .filter(|token| !token.is_empty())
    }

    /// Discovery instructions for every registered provider, in
    /// registration order.
    pub fn discover_instructions(&self) -> Vec<(String, DiscoverInstruction)> {
        self.providers
            .iter()
            .map(|provider| (provider.name.clone(), self.discover_instruction(provider)))
            .collect()
    }

    fn discover_instruction(&self, provider: &Provider) -> DiscoverInstruction {
        match provider.provider_type {
            ProviderType::Http => DiscoverInstruction::Http {
                url: format!("{}/tools", provider.endpoint.trim_end_matches('/')),
                bearer: Self::bearer(provider),
            },
            ProviderType::Cli => {
                let (program, args) = self.cli_base(provider);
                let mut args = args;
                args.push("--list-tools".to_string());
                DiscoverInstruction::Cli { program, args }
            }
            ProviderType::Mcp => DiscoverInstruction::Mcp {
                command: provider.mcp_command(),
            },
        }
    }

    fn cli_base(&self, provider: &Provider) -> (String, Vec<String>) {
        if !provider.command.is_empty() {
            (provider.command[0].clone(), provider.command[1..].to_vec())
        } else {
            (provider.endpoint.clone(), Vec::new())
        }
    }

    /// Record a discovery result; replaces that provider's index entries.
    pub fn record_discovery(&mut self, provider: &str, tools: Vec<ToolSchema>) {
        self.index.retain(|tool| tool.provider != provider);
        self.index.extend(tools.into_iter().map(|mut tool| {
            tool.provider = provider.to_string();
            tool
        }));
    }

    pub fn tool_count(&self) -> usize {
        self.index.len()
    }

    pub fn tools(&self) -> &[ToolSchema] {
        &self.index
    }

    /// Search the index for tools matching a keyword (case-insensitive);
    /// name matches rank before description matches.
    pub fn search_tools(&self, keyword: &str) -> Vec<&ToolSchema> {
        let keyword = keyword.to_lowercase();
        let mut name_matches = Vec::new();
        let mut description_matches = Vec::new();
        for tool in &self.index {
            if tool.name.to_lowercase().contains(&keyword) {
                name_matches.push(tool);
            } else if tool.description.to_lowercase().contains(&keyword) {
                description_matches.push(tool);
            }
        }
        name_matches.extend(description_matches);
        name_matches
    }

    /// Find a tool by exact name.
    pub fn find_tool(&self, name: &str) -> Option<&ToolSchema> {
        self.index.iter().find(|tool| tool.name == name)
    }

    /// The call instruction for `provider`'s `tool`: the provider's
    /// native transport shape with the endpoint, auth, and arguments
    /// substituted in. Namespaced names pin the provider, so lookup is
    /// scoped — a same-named tool on another provider never shadows it.
    pub fn call_instruction(
        &self,
        provider: &str,
        tool: &str,
        args: Value,
    ) -> Result<CallInstruction, UtcpError> {
        let known = self
            .index
            .iter()
            .any(|entry| entry.provider == provider && entry.name == tool);
        if !known {
            return Err(UtcpError::UnknownTool(format!("{provider}/{tool}")));
        }
        let provider = self
            .provider(provider)
            .ok_or_else(|| UtcpError::UnknownProvider(provider.to_string()))?;
        let instruction = match provider.provider_type {
            ProviderType::Http => CallInstruction::Http {
                url: format!("{}/tools/{}", provider.endpoint.trim_end_matches('/'), tool),
                bearer: Self::bearer(provider),
                body: args,
            },
            ProviderType::Cli => {
                let (program, mut argv) = self.cli_base(provider);
                argv.push(tool.to_string());
                argv.push(args.to_string());
                CallInstruction::Cli {
                    program,
                    args: argv,
                }
            }
            ProviderType::Mcp => CallInstruction::Mcp {
                command: provider.mcp_command(),
                tool: tool.to_string(),
                args,
            },
        };
        Ok(instruction)
    }
}

#[cfg(test)]
mod tests;
