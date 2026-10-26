//! The Orexis daemon: sessions over a Unix-socket NDJSON protocol.

mod approvals;
mod connection;
mod handlers;
mod proto;
mod server;
mod session;

pub use proto::{Artifact, ClientMsg, FileEntry, ServerMsg, SessionInfo, SettingsInfo};
pub use server::{Daemon, DaemonConfig};
