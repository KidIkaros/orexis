//! The team domain's data types (M8, ADR-0020): board tasks, bus
//! messages, leases. Plain data; the ports carry the traits.

use serde::{Deserialize, Serialize};

/// A board task's stage. `queued → admitted → running`, `blocked` when
/// edges wait, terminal `done` / `failed`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    #[default]
    Queued,
    Admitted,
    Running,
    Blocked,
    Done,
    Failed,
}

/// A durable task on the board: owner, required strategy, blocking edges.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoardTask {
    pub id: String,
    pub title: String,
    pub required_strategy: String,
    pub owner: Option<String>,
    pub status: TaskStatus,
    /// Task ids that must be `done` before this one is ready.
    pub depends_on: Vec<String>,
    pub seq: u64,
}

/// A bus message: direct (to one worker) or room (to the team).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BusMessage {
    pub seq: u64,
    pub from: String,
    /// `None` is the room.
    pub to: Option<String>,
    pub text: String,
}

/// A lease on a workspace path: an owner and an expiry; a stale lease is
/// stealable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lease {
    pub path: String,
    pub owner: String,
    /// Unix seconds; expired means stealable.
    pub expires_at: u64,
}
