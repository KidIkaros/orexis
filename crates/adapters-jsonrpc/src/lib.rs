//! JSON-RPC 2.0 envelope, ndjson framing, SSE framing, and correlation.
//!
//! Shared adapter infrastructure (ADR-0005): pure data and pure functions,
//! no I/O, no domain types, no runtime. MCP, ACP, and A2A speak JSON-RPC
//! 2.0 over ndjson or SSE; each protocol adapter owns its methods and uses
//! this crate for the envelope.

pub mod correlate;
pub mod framing;
pub mod message;
pub mod sse;

pub use correlate::Correlations;
pub use framing::{decode_str, encode_line};
pub use message::{
    parse_line, ErrorObject, Message, Notification, ParseError, Request, RequestId, Response,
};
pub use sse::{SseDecoder, SseEvent};
