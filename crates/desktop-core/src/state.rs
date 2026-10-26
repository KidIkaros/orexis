//! The desktop state model and daemon bridge, independent of Tauri.

use orexis_core::SessionEvent;
use orexis_protocol::{ServerMsg, SessionInfo};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Connection {
    Connecting,
    Attached,
    Disconnected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum LeftTab {
    Sessions,
    Board,
    Fleet,
    Artifacts,
    Files,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum RightTab {
    Approvals,
    Diff,
    Preview,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Status {
    Empty,
    Running,
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
pub enum DesktopEntry {
    User(String),
    Assistant(String),
    Tool {
        call_id: String,
        name: String,
        target: String,
        ok: bool,
        detail: String,
    },
    Approval(Approval),
    Error(String),
    Info(String),
}

#[derive(Debug, Clone, Serialize)]
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
            ServerMsg::ApprovalRequest { id, tool, args } => {
                let approval = Approval { id, tool, args };
                self.pending = Some(approval.clone());
                self.transcript.push(DesktopEntry::Approval(approval));
                self.right = RightTab::Approvals;
                self.status = Status::Approval;
            }
            ServerMsg::Busy { .. } => self.status = Status::Running,
            ServerMsg::Queued { .. } => self.status = Status::Queued,
            ServerMsg::Sessions { sessions } => self.sessions = sessions,
            ServerMsg::Error { message } => {
                self.status = Status::Error;
                self.transcript.push(DesktopEntry::Error(message));
            }
        }
    }

    fn apply_event(&mut self, event: &SessionEvent) {
        match event {
            SessionEvent::UserMessage { text } => {
                self.transcript.push(DesktopEntry::User(text.clone()));
                self.status = Status::Running;
            }
            SessionEvent::ModelResponse { text, .. } if !text.is_empty() => {
                self.transcript.push(DesktopEntry::Assistant(text.clone()));
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
                self.transcript
                    .push(DesktopEntry::Info(format!("done: {summary}")));
            }
            SessionEvent::Error { message } => {
                self.status = Status::Error;
                self.transcript.push(DesktopEntry::Error(message.clone()));
            }
            SessionEvent::Reprompt { note } => {
                self.transcript
                    .push(DesktopEntry::Info(format!("reprompt: {note}")));
            }
            SessionEvent::Compaction { .. } => {
                self.transcript
                    .push(DesktopEntry::Info("compacted earlier context".to_string()));
            }
            SessionEvent::Checkpoint { id, .. } => {
                self.transcript
                    .push(DesktopEntry::Info(format!("checkpoint {id}")));
            }
            SessionEvent::Verify { passed, .. } => {
                self.transcript.push(DesktopEntry::Info(if *passed {
                    "verify passed".to_string()
                } else {
                    "verify failed".to_string()
                }));
            }
            SessionEvent::Interrupted { reason } => {
                self.transcript
                    .push(DesktopEntry::Info(format!("interrupted: {reason}")));
            }
            SessionEvent::Info { message } => {
                self.transcript.push(DesktopEntry::Info(message.clone()));
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
            DesktopEntry::User("refactor".to_string())
        );
        assert_eq!(
            state.transcript[1],
            DesktopEntry::Assistant("reading".to_string())
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
}
