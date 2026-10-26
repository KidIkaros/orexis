//! The NDJSON client/server protocol shared by the daemon and its clients.

use orexis_core::SessionEvent;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ClientMsg {
    Attach {
        session: Option<String>,
    },
    Prompt {
        text: String,
        #[serde(default)]
        skip_verify: bool,
    },
    Approve {
        id: u64,
        yes: bool,
        #[serde(default)]
        always: bool,
    },
    ClearOverrides,
    ListDir {
        path: String,
    },
    ReadFile {
        path: String,
    },
    Artifacts,
    Interrupt,
    Undo,
    Diff,
    ListSessions,
    Detach,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ServerMsg {
    Attached {
        session_id: String,
    },
    Event {
        seq: u64,
        replay: bool,
        event: SessionEvent,
    },
    Delta {
        text: String,
    },
    ApprovalRequest {
        id: u64,
        #[serde(default)]
        session_id: String,
        tool: String,
        args: String,
    },
    Busy {
        session_id: String,
    },
    Queued {
        session_id: String,
        position: usize,
    },
    Sessions {
        sessions: Vec<SessionInfo>,
    },
    Dir {
        path: String,
        entries: Vec<FileEntry>,
    },
    File {
        path: String,
        content: String,
        truncated: bool,
    },
    Artifacts {
        items: Vec<Artifact>,
    },
    Diff {
        diff: String,
    },
    Error {
        message: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Artifact {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub detail: String,
}
