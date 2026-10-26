//! Snapshot DTOs mirrored from desktop-core.

use serde::Deserialize;

#[derive(Clone, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub status: String,
}

#[derive(Clone, Deserialize)]
pub struct Approval {
    pub id: u64,
    pub tool: String,
    pub args: String,
}

#[derive(Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Entry {
    User {
        text: String,
    },
    Assistant {
        text: String,
    },
    Tool {
        name: String,
        target: String,
        ok: bool,
        detail: String,
    },
    Approval {
        tool: String,
        args: String,
    },
    Error {
        message: String,
    },
    Info {
        message: String,
    },
}

#[derive(Clone, Deserialize)]
pub struct Snapshot {
    pub connection: String,
    pub session: Option<String>,
    pub sessions: Vec<SessionInfo>,
    pub transcript: Vec<Entry>,
    pub pending: Option<Approval>,
    pub status: String,
    pub live: Option<String>,
    pub diff: Option<String>,
}

impl Default for Snapshot {
    fn default() -> Self {
        Self {
            connection: "connecting".to_string(),
            session: None,
            sessions: Vec::new(),
            transcript: Vec::new(),
            pending: None,
            status: "empty".to_string(),
            live: None,
            diff: None,
        }
    }
}

pub fn status_label(status: &str) -> &str {
    match status {
        "empty" => "ready",
        "running" => "running",
        "streaming" => "streaming",
        "approval" => "approval",
        "error" => "error",
        "budget" => "budget",
        "disconnected" => "disconnected",
        "queued" => "queued",
        "compacting" => "compacting",
        _ => status,
    }
}
