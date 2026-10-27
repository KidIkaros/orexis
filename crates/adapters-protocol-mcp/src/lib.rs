//! MCP client core — sans-io (ADR-0017 §1).
//!
//! Ported from Kindred `src/mcp.rs` (MIT, one-way ledger): the blocking
//! stdio client becomes a state machine driven by the caller. The core
//! never spawns a process or opens a socket; shells (M7-03 stdio, M7-05
//! streamable HTTP) own I/O and feed lines in and out.
//!
//! The port fixes Kindred's interleaving bug: its blocking `request()`
//! read one line and assumed it was the response, so a server-initiated
//! notification or request corrupted the stream. Here every response is
//! correlated by id and everything else becomes an event.

pub mod core;
pub mod error;
pub mod peer;
pub mod stdio;
pub mod tools;
pub mod types;

pub use core::{Event, McpCore};
pub use error::McpError;
pub use peer::StdioPeerFactory;
pub use stdio::{McpServerConfig, McpStdioClient};
pub use tools::{load_server_configs, McpTools};
pub use types::{
    ClientInfo, Initialization, McpPrompt, McpPromptArgument, McpResource, ResourceContent,
    ServerCapabilities, ServerInfo, ToolCallResult, ToolContent,
};
