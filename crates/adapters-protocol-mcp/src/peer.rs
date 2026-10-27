//! The stdio client behind consumer-side peer ports.
//!
//! UTCP's MCP transport connects to servers by argv; `StdioPeerFactory`
//! adapts `McpStdioClient` to that shape so the UTCP adapter composes
//! this client without depending on the crate (ADR-0005).

use std::collections::BTreeMap;
use std::time::Duration;

use async_trait::async_trait;
use orexis_core::ToolSpec;
use orexis_ports::{PeerFactory, PeerOutput, PeerSession, PortError, PortResult};
use serde_json::Value;

use crate::stdio::{McpServerConfig, McpStdioClient};
use crate::tools::{render_content, MAX_PAGES};

/// Connects MCP peer servers over stdio for use as a UTCP transport.
pub struct StdioPeerFactory {
    pub timeout: Duration,
}

#[async_trait]
impl PeerFactory for StdioPeerFactory {
    async fn connect(&self, command: &[String]) -> PortResult<Box<dyn PeerSession>> {
        let Some((program, args)) = command.split_first() else {
            return Err(PortError("mcp provider has an empty command".into()));
        };
        let config = McpServerConfig {
            name: "utcp".to_string(),
            command: program.clone(),
            args: args.to_vec(),
            env: BTreeMap::new(),
        };
        let client = McpStdioClient::spawn(&config, self.timeout)
            .await
            .map_err(|error| PortError(error.to_string()))?;
        Ok(Box::new(client))
    }
}

#[async_trait]
impl PeerSession for McpStdioClient {
    async fn tools(&mut self) -> PortResult<Vec<ToolSpec>> {
        let mut specs = Vec::new();
        let mut cursor = None;
        for _ in 0..MAX_PAGES {
            let (tools, next) = self
                .list_tools(cursor.take())
                .await
                .map_err(|error| PortError(error.to_string()))?;
            specs.extend(tools);
            match next {
                Some(next) => cursor = Some(next),
                None => break,
            }
        }
        Ok(specs)
    }

    async fn call(&mut self, name: &str, args: &str) -> PortResult<PeerOutput> {
        let arguments = if args.trim().is_empty() {
            None
        } else {
            let value: Value = serde_json::from_str(args)
                .map_err(|error| PortError(format!("invalid tool arguments: {error}")))?;
            Some(value)
        };
        let result = self
            .call_tool(name, arguments)
            .await
            .map_err(|error| PortError(error.to_string()))?;
        Ok(PeerOutput {
            text: render_content(&result.content),
            is_error: result.is_error,
        })
    }
}
