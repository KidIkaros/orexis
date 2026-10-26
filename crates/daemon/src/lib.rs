//! The Orexis daemon: sessions over a Unix-socket NDJSON protocol.

mod approvals;
mod proto;
mod server;
mod session;

pub use proto::{ClientMsg, ServerMsg, SessionInfo};
pub use server::{Daemon, DaemonConfig};
