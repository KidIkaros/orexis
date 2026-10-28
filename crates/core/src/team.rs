//! The team domain's data types (M8, ADR-0020): board tasks, bus
//! messages, leases. Plain data; the ports carry the traits.

use serde::{Deserialize, Serialize};

/// The board's task cap (M8 limits: 256; a team may tighten, never
/// exceed).
pub const BOARD_CAP: usize = 256;

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

impl TaskStatus {
    /// The label the panels and the board tools share.
    pub fn label(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Admitted => "admitted",
            Self::Running => "running",
            Self::Blocked => "blocked",
            Self::Done => "done",
            Self::Failed => "failed",
        }
    }
}

/// One task row — the shared rendering for the board panel, the fleet
/// panel, and the model's board tools (M8-11).
pub fn render_task(task: &BoardTask) -> String {
    let owner = task.owner.as_deref().unwrap_or("-");
    let mut line = format!(
        "{} [{}] {} — {} (owner {})",
        task.id,
        task.status.label(),
        task.required_strategy,
        task.title,
        owner
    );
    if !task.depends_on.is_empty() {
        line.push_str(&format!(" after {}", task.depends_on.join(", ")));
    }
    line
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

/// A strategy's toolset: tool names the strategy may call. An empty set
/// means no restriction (the gate still applies). Matching is by exact
/// name or namespace prefix (`mcp/`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolsetFilter {
    pub allowed: Vec<String>,
}

impl ToolsetFilter {
    /// No restriction.
    pub fn open() -> Self {
        Self {
            allowed: Vec::new(),
        }
    }

    pub fn allow(names: &[&str]) -> Self {
        Self {
            allowed: names.iter().map(|name| name.to_string()).collect(),
        }
    }

    /// True when the tool may be called. An empty allowlist is open.
    pub fn permits(&self, tool: &str) -> bool {
        if self.allowed.is_empty() {
            return true;
        }
        self.allowed
            .iter()
            .any(|allowed| tool == allowed || tool.starts_with(&format!("{allowed}/")))
    }
}
