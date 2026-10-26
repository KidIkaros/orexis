//! MCP core errors.

use orexis_adapters_jsonrpc::ParseError;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum McpError {
    #[error("the client is not initialized")]
    NotReady,
    #[error("initialize was already sent")]
    AlreadyInitializing,
    #[error("malformed wire: {0}")]
    Parse(#[from] ParseError),
    #[error("invalid initialize result: {0}")]
    Initialization(String),
    #[error("transport: {0}")]
    Transport(String),
    #[error("timed out waiting for the server")]
    Timeout,
}
