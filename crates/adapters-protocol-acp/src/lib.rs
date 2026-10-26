//! ACP (Agent Client Protocol) server side — sans-io (ADR-0017 §1, §5).
//!
//! Editors and hosts connect in over stdio; this crate owns the ACP
//! method dispatch, the response builders, and the bridge from the
//! daemon's `ServerMsg` stream to `session/update` notifications. The
//! stdio loop and the session machinery live in the daemon (app layer).

pub mod bridge;
pub mod dispatch;

pub use bridge::{stop_reason_from, update_from};
pub use dispatch::{
    client_request, initialize_response, method_not_found, outcome_from, parse_incoming,
    permission_request, prompt_response, session_new_response, update_notification, AcpError,
    AcpIncoming,
};
pub use orexis_adapters_jsonrpc::{encode_line, Message};

/// The ACP protocol version this server speaks.
pub const PROTOCOL_VERSION: u64 = 1;
