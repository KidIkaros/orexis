//! UTCP client core — sans-io (ADR-0017 §1, §4).
//!
//! Ported from Kindred `src/utcp.rs` (MIT, one-way ledger): a provider is
//! a JSON description of a tool source and how to reach it; the client
//! indexes tool schemas locally and dispatches calls over the provider's
//! native transport. Here the core stops at *instructions*: it parses
//! manuals, indexes and searches tools, and builds the call/discovery
//! instructions the transport shells (M7-09) execute. No I/O here.

pub mod core;
pub mod manual;

pub use core::{CallInstruction, DiscoverInstruction, ToolSchema, UtcpCore};
pub use manual::{parse_providers, Provider, ProviderType, UtcpError};
