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
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
}

#[derive(Clone, Deserialize)]
pub struct FilePreview {
    pub path: String,
    pub content: String,
    pub truncated: bool,
}

#[derive(Clone, Deserialize)]
pub struct Artifact {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub detail: String,
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
    pub files: Vec<FileEntry>,
    pub files_path: String,
    pub preview: Option<FilePreview>,
    pub artifacts: Vec<Artifact>,
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
            files: Vec::new(),
            files_path: ".".to_string(),
            preview: None,
            artifacts: Vec::new(),
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

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LeftTab {
    Sessions,
    Board,
    Fleet,
    Artifacts,
    Files,
}
