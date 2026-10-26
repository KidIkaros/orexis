//! The desktop state model and daemon bridge, independent of Tauri.

use orexis_core::SessionEvent;
use orexis_protocol::{Artifact, FileEntry, ServerMsg, SessionInfo, SettingsInfo};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Connection {
    Connecting,
    Attached,
    Disconnected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LeftTab {
    Sessions,
    Board,
    Fleet,
    Artifacts,
    Files,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RightTab {
    Approvals,
    Diff,
    Preview,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Empty,
    Running,
    Streaming,
    Approval,
    Error,
    Budget,
    Queued,
    Compacting,
    Disconnected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Approval {
    pub id: u64,
    pub tool: String,
    pub args: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DesktopEntry {
    User {
        text: String,
    },
    Assistant {
        text: String,
    },
    Tool {
        call_id: String,
        name: String,
        target: String,
        ok: bool,
        detail: String,
    },
    Approval {
        id: u64,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FilePreview {
    pub path: String,
    pub content: String,
    pub truncated: bool,
}

pub struct AppState {
    pub connection: Connection,
    pub session: Option<String>,
    pub sessions: Vec<SessionInfo>,
    pub transcript: Vec<DesktopEntry>,
    pub pending: Option<Approval>,
    pub left: LeftTab,
    pub right: RightTab,
    pub settings_open: bool,
    pub palette_open: bool,
    pub status: Status,
    pub live: Option<String>,
    pub diff: Option<String>,
    pub files: Vec<FileEntry>,
    pub files_path: String,
    pub preview: Option<FilePreview>,
    pub artifacts: Vec<Artifact>,
    pub settings: Option<SettingsInfo>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            connection: Connection::Connecting,
            session: None,
            sessions: Vec::new(),
            transcript: Vec::new(),
            pending: None,
            left: LeftTab::Sessions,
            right: RightTab::Approvals,
            settings_open: false,
            palette_open: false,
            status: Status::Empty,
            live: None,
            diff: None,
            files: Vec::new(),
            files_path: ".".to_string(),
            preview: None,
            artifacts: Vec::new(),
            settings: None,
        }
    }
}

impl AppState {
    pub fn apply(&mut self, message: ServerMsg) {
        match message {
            ServerMsg::Attached { session_id } => {
                self.connection = Connection::Attached;
                self.session = Some(session_id);
                self.status = Status::Empty;
            }
            ServerMsg::Event { event, .. } => self.apply_event(&event),
            ServerMsg::Delta { text } => self.apply_delta(&text),
            ServerMsg::ApprovalRequest { id, tool, args, .. } => {
                let approval = Approval { id, tool, args };
                self.pending = Some(approval.clone());
                self.transcript.push(DesktopEntry::Approval {
                    id: approval.id,
                    tool: approval.tool,
                    args: approval.args,
                });
                self.right = RightTab::Approvals;
                self.status = Status::Approval;
            }
            ServerMsg::Busy { .. } => self.status = Status::Running,
            ServerMsg::Queued { .. } => self.status = Status::Queued,
            ServerMsg::Sessions { sessions } => self.sessions = sessions,
            ServerMsg::Diff { diff } => {
                self.diff = Some(diff);
                self.right = RightTab::Diff;
            }
            ServerMsg::Dir { path, entries } => {
                self.files = entries;
                self.files_path = path;
            }
            ServerMsg::File {
                path,
                content,
                truncated,
            } => {
                self.preview = Some(FilePreview {
                    path,
                    content,
                    truncated,
                });
                self.right = RightTab::Preview;
            }
            ServerMsg::Artifacts { items } => self.artifacts = items,
            ServerMsg::Settings { settings } => self.settings = Some(settings),
            ServerMsg::Audit { .. } => {}
            ServerMsg::SelfView { .. } => {}
            ServerMsg::Error { message } => {
                self.status = Status::Error;
                self.transcript.push(DesktopEntry::Error { message });
            }
        }
    }

    pub fn apply_delta(&mut self, text: &str) {
        self.live.get_or_insert_with(String::new).push_str(text);
        self.status = Status::Streaming;
    }

    fn apply_event(&mut self, event: &SessionEvent) {
        match event {
            SessionEvent::UserMessage { text } => {
                self.transcript
                    .push(DesktopEntry::User { text: text.clone() });
                self.status = Status::Running;
            }
            SessionEvent::ModelResponse { text, .. } => {
                self.live = None;
                if !text.is_empty() {
                    self.transcript
                        .push(DesktopEntry::Assistant { text: text.clone() });
                }
                self.status = Status::Running;
            }
            SessionEvent::ToolCall {
                call_id,
                name,
                args,
            } => {
                self.transcript.push(DesktopEntry::Tool {
                    call_id: call_id.clone(),
                    name: name.clone(),
                    target: args.clone(),
                    ok: false,
                    detail: String::new(),
                });
                self.status = Status::Running;
            }
            SessionEvent::Observation { name, output, .. } => {
                for entry in self.transcript.iter_mut().rev() {
                    if let DesktopEntry::Tool {
                        name: tool,
                        ok,
                        detail,
                        ..
                    } = entry
                    {
                        if tool == name && !*ok {
                            *ok = true;
                            *detail = output.clone();
                            break;
                        }
                    }
                }
            }
            SessionEvent::Done { summary } => {
                self.live = None;
                self.status = Status::Empty;
                self.transcript.push(DesktopEntry::Info {
                    message: format!("done: {summary}"),
                });
            }
            SessionEvent::Error { message } => {
                self.live = None;
                self.status = Status::Error;
                self.transcript.push(DesktopEntry::Error {
                    message: message.clone(),
                });
            }
            SessionEvent::Reprompt { note } => {
                self.live = None;
                self.transcript.push(DesktopEntry::Info {
                    message: format!("reprompt: {note}"),
                });
            }
            SessionEvent::Compaction { .. } => {
                self.transcript.push(DesktopEntry::Info {
                    message: "compacted earlier context".to_string(),
                });
            }
            SessionEvent::Checkpoint { id, .. } => {
                self.transcript.push(DesktopEntry::Info {
                    message: format!("checkpoint {id}"),
                });
            }
            SessionEvent::Verify { passed, .. } => {
                self.transcript.push(DesktopEntry::Info {
                    message: if *passed {
                        "verify passed".to_string()
                    } else {
                        "verify failed".to_string()
                    },
                });
            }
            SessionEvent::ApprovalDecision {
                tool,
                approved,
                always,
                ..
            } => {
                self.transcript.push(DesktopEntry::Info {
                    message: format!(
                        "approval: {tool} {} always={always}",
                        if *approved { "approved" } else { "denied" }
                    ),
                });
            }
            SessionEvent::Recovered { reason } => {
                self.live = None;
                self.status = Status::Empty;
                self.transcript.push(DesktopEntry::Info {
                    message: format!("recovered: {reason}"),
                });
            }
            SessionEvent::Interrupted { reason } => {
                self.live = None;
                self.transcript.push(DesktopEntry::Info {
                    message: format!("interrupted: {reason}"),
                });
            }
            SessionEvent::Info { message } => {
                if message.starts_with("restored checkpoint") {
                    self.live = None;
                    self.diff = None;
                }
                self.transcript.push(DesktopEntry::Info {
                    message: message.clone(),
                });
            }
            _ => {}
        }
    }

    pub fn set_disconnected(&mut self) {
        self.connection = Connection::Disconnected;
        self.status = Status::Disconnected;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(event: SessionEvent) -> ServerMsg {
        ServerMsg::Event {
            seq: 1,
            replay: false,
            event,
        }
    }

    #[test]
    fn approval_request_sets_pending_and_right_tab() {
        let mut state = AppState::default();
        state.apply(ServerMsg::ApprovalRequest {
            id: 7,
            session_id: "s1".to_string(),
            tool: "shell".to_string(),
            args: "ls".to_string(),
        });
        assert_eq!(state.pending.as_ref().map(|approval| approval.id), Some(7));
        assert_eq!(state.right, RightTab::Approvals);
        assert_eq!(state.status, Status::Approval);
    }

    #[test]
    fn tool_observation_pairs_with_the_call() {
        let mut state = AppState::default();
        state.apply(event(SessionEvent::ToolCall {
            call_id: "c1".to_string(),
            name: "read_file".to_string(),
            args: "a.txt".to_string(),
        }));
        state.apply(event(SessionEvent::Observation {
            call_id: "c1".to_string(),
            name: "read_file".to_string(),
            output: "ok".to_string(),
        }));
        match &state.transcript[0] {
            DesktopEntry::Tool { ok, detail, .. } => {
                assert!(ok);
                assert_eq!(detail, "ok");
            }
            other => panic!("unexpected entry: {other:?}"),
        }
    }

    #[test]
    fn event_stream_builds_the_transcript() {
        let mut state = AppState::default();
        state.apply(event(SessionEvent::UserMessage {
            text: "refactor".to_string(),
        }));
        state.apply(event(SessionEvent::ModelResponse {
            text: "reading".to_string(),
            actions: Vec::new(),
        }));
        assert_eq!(state.transcript.len(), 2);
        assert_eq!(
            state.transcript[0],
            DesktopEntry::User {
                text: "refactor".to_string()
            }
        );
        assert_eq!(
            state.transcript[1],
            DesktopEntry::Assistant {
                text: "reading".to_string()
            }
        );
    }

    #[test]
    fn disconnect_is_visible() {
        let mut state = AppState::default();
        state.set_disconnected();
        assert_eq!(state.connection, Connection::Disconnected);
        assert_eq!(state.status, Status::Disconnected);
    }

    #[test]
    fn same_name_tools_pair_by_call_id() {
        let mut state = AppState::default();
        state.apply(event(SessionEvent::ToolCall {
            call_id: "c1".to_string(),
            name: "read_file".to_string(),
            args: "a".to_string(),
        }));
        state.apply(event(SessionEvent::ToolCall {
            call_id: "c2".to_string(),
            name: "read_file".to_string(),
            args: "b".to_string(),
        }));
        state.apply(event(SessionEvent::Observation {
            call_id: "c2".to_string(),
            name: "read_file".to_string(),
            output: "second".to_string(),
        }));
        let DesktopEntry::Tool {
            call_id,
            ok,
            detail,
            ..
        } = &state.transcript[1]
        else {
            panic!("expected tool");
        };
        assert_eq!(call_id, "c2");
        assert!(ok);
        assert_eq!(detail, "second");
        let DesktopEntry::Tool { ok, .. } = &state.transcript[0] else {
            panic!("expected tool");
        };
        assert!(!ok);
    }

    #[test]
    fn delta_stream_appends_and_final_clears() {
        let mut state = AppState::default();
        state.apply(ServerMsg::Delta {
            text: "he".to_string(),
        });
        state.apply(ServerMsg::Delta {
            text: "llo".to_string(),
        });
        assert_eq!(state.live.as_deref(), Some("hello"));

        state.apply(event(SessionEvent::ModelResponse {
            text: "hello".to_string(),
            actions: Vec::new(),
        }));
        assert!(state.live.is_none());
        assert!(state
            .transcript
            .iter()
            .any(|entry| matches!(entry, DesktopEntry::Assistant { text } if text == "hello")));
    }

    #[test]
    fn apply_diff_sets_the_right_tab() {
        let mut state = AppState::default();
        state.apply(ServerMsg::Diff {
            diff: "--- a/a.txt\n+++ b/a.txt\n-one\n+two\n".to_string(),
        });
        assert!(state.diff.as_deref().unwrap().contains("+two"));
        assert_eq!(state.right, RightTab::Diff);
    }

    #[test]
    fn delta_sets_streaming_status() {
        let mut state = AppState::default();
        state.apply(ServerMsg::Delta {
            text: "hi".to_string(),
        });
        assert_eq!(state.status, Status::Streaming);
    }

    #[test]
    fn error_clears_the_live_buffer() {
        let mut state = AppState::default();
        state.apply(ServerMsg::Delta {
            text: "hi".to_string(),
        });
        state.apply(event(SessionEvent::Error {
            message: "boom".to_string(),
        }));
        assert!(state.live.is_none());
        assert_eq!(state.status, Status::Error);
    }

    #[test]
    fn undo_info_clears_the_diff() {
        let mut state = AppState::default();
        state.apply(ServerMsg::Diff {
            diff: "--- a\n+++ b\n".to_string(),
        });
        state.apply(event(SessionEvent::Info {
            message: "restored checkpoint cp-1".to_string(),
        }));
        assert!(state.diff.is_none());
    }

    #[test]
    fn ui_snapshot_shape_is_stable() {
        let entry = serde_json::to_value(DesktopEntry::User {
            text: "hi".to_string(),
        })
        .expect("entry");
        assert_eq!(entry["kind"], "user");
        assert_eq!(
            serde_json::to_value(Status::Streaming).expect("status"),
            "streaming"
        );
        assert_eq!(
            serde_json::to_value(Connection::Disconnected).expect("connection"),
            "disconnected"
        );
    }

    #[test]
    fn file_messages_update_state() {
        let mut state = AppState::default();
        state.apply(ServerMsg::Dir {
            path: "src".to_string(),
            entries: vec![FileEntry {
                name: "lib.rs".to_string(),
                path: "src/lib.rs".to_string(),
                is_dir: false,
                size: 12,
            }],
        });
        assert_eq!(state.files_path, "src");
        assert_eq!(state.files.len(), 1);

        state.apply(ServerMsg::File {
            path: "src/lib.rs".to_string(),
            content: "hello".to_string(),
            truncated: false,
        });
        assert_eq!(
            state
                .preview
                .as_ref()
                .map(|preview| preview.content.as_str()),
            Some("hello")
        );
        assert_eq!(state.right, RightTab::Preview);

        state.apply(ServerMsg::Artifacts {
            items: vec![Artifact {
                id: "a".to_string(),
                kind: "file".to_string(),
                title: "a".to_string(),
                detail: String::new(),
            }],
        });
        assert_eq!(state.artifacts.len(), 1);
    }

    #[test]
    fn settings_update_state() {
        let mut state = AppState::default();
        state.apply(ServerMsg::Settings {
            settings: SettingsInfo {
                profile: "openai".to_string(),
                model: "gpt-5".to_string(),
                plan_mode: false,
                permissions: vec!["shell: ask".to_string()],
                privacy: vec!["no telemetry".to_string()],
            },
        });
        let settings = state.settings.expect("settings");
        assert_eq!(settings.profile, "openai");
        assert!(settings
            .privacy
            .iter()
            .any(|line| line.contains("no telemetry")));
    }

    #[test]
    fn recovered_clears_live_and_sets_ready() {
        let mut state = AppState::default();
        state.apply(ServerMsg::Delta {
            text: "partial".to_string(),
        });
        state.apply(event(SessionEvent::Recovered {
            reason: "daemon restarted".to_string(),
        }));
        assert!(state.live.is_none());
        assert_eq!(state.status, Status::Empty);
        assert!(state.transcript.iter().any(|entry| matches!(
            entry,
            DesktopEntry::Info { message } if message.contains("recovered")
        )));
    }
}
