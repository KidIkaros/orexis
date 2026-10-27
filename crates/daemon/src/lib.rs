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

pub mod cli;
pub use approvals::{ApprovalHub, MemoryOverrides};
pub mod team;
pub use acp::AcpServer;
pub use cli::{acp_main, serve_main};
pub use env::daemon_config_from_env;
pub use proto::{Artifact, ClientMsg, FileEntry, ServerMsg, SessionInfo, SettingsInfo};
pub use server::{Daemon, DaemonConfig};
pub use session::{SessionRegistry, SessionRuntime};
pub use team::{spawn_worker_turn, WorkerOutcome, WorkerTurn};
