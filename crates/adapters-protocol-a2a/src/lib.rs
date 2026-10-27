//! A2A client — sans-io core plus an HTTP shell (ADR-0017 §1, §6).
//!
//! A2A is JSON-RPC 2.0 with its own methods and objects: a remote agent
//! publishes an agent card, the client hands it work with `message/send`,
//! and reads the resulting task back with `tasks/get`. The core owns the
//! shapes; the shell owns the socket.
//!
//! The client is client-only in M7: Orexis calls remote agents, it does
//! not serve them. An A2A server is team-mode-shaped work (ADR-0017 §6).

pub mod core;
pub mod http;
pub mod tools;

pub use core::{
    parse_card, read_message, A2aCore, A2aError, A2aEvent, AgentCard, AgentSkill, MethodNaming,
    Outcome, Task, TaskState,
};
pub use http::{A2aClient, A2aClientConfig};
pub use tools::{load_peer_configs, A2aPeer, A2aTools};
