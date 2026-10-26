//! The Orexis daemon: sessions over a Unix-socket NDJSON protocol, and the
//! ACP stdio server.

mod acp;
mod approvals;
mod connection;
mod env;
mod handlers;
mod proto;
mod server;
mod session;

pub use acp::AcpServer;
pub use env::daemon_config_from_env;
pub use proto::{Artifact, ClientMsg, FileEntry, ServerMsg, SessionInfo, SettingsInfo};
pub use server::{Daemon, DaemonConfig};
