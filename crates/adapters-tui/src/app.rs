//! The TUI application state and non-key methods.

use std::cell::RefCell;

use orexis_core::SessionEvent;
use orexis_protocol::ServerMsg;
use ratatui::Frame;

use crate::render;
use crate::state::{
    Entry, Focus, Overlay, PlanTask, ToolRow, TranscriptCache, TuiAction, TuiState,
};

pub struct TuiApp {
    pub state: TuiState,
    pub session: String,
    pub worker: String,
    pub model: String,
    pub provider: String,
    pub context_pct: u8,
    pub cost: String,
    pub plan_mode: bool,
    pub queue: usize,
    pub entries: Vec<Entry>,
    pub composer: String,
    pub focus: Focus,
    pub overlay: Option<Overlay>,
    pub scroll: usize,
    pub selected: usize,
    pub color: bool,
    pub bottom_sheet: Option<String>,
    pub error: Option<String>,
    pub budget: Option<(u64, u64)>,
    pub queued: Option<usize>,
    pub live: Option<String>,
    pub self_text: Option<String>,
    pub pending_approval: Option<u64>,
    pub palette_query: String,
    pub palette_selected: usize,
    pub search_query: String,
    pub revision: u64,
    pub(crate) cache: RefCell<TranscriptCache>,
}

impl Default for TuiApp {
    fn default() -> Self {
        Self {
            state: TuiState::Empty,
            session: "s1".to_string(),
            worker: "1/1".to_string(),
            model: "unset".to_string(),
            provider: "unset".to_string(),
            context_pct: 0,
            cost: "$0.00".to_string(),
            plan_mode: false,
            queue: 0,
            entries: Vec::new(),
            composer: String::new(),
            focus: Focus::Composer,
            overlay: None,
            scroll: 0,
            selected: 0,
            color: true,
            bottom_sheet: None,
            error: None,
            budget: None,
            queued: None,
            live: None,
            self_text: None,
            pending_approval: None,
            palette_query: String::new(),
            palette_selected: 0,
            search_query: String::new(),
            revision: 0,
            cache: RefCell::new(TranscriptCache {
                revision: u64::MAX,
                lines: Vec::new(),
            }),
        }
    }
}

impl TuiApp {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_state(&mut self, state: TuiState) {
        self.state = state;
        self.revision += 1;
    }

    pub fn push_event(&mut self, event: &SessionEvent) {
        match event {
            SessionEvent::UserMessage { text } => {
                self.entries.push(Entry::User(text.clone()));
                self.state = TuiState::Streaming;
            }
            SessionEvent::ModelResponse { text, .. } => {
                self.live = None;
                if !text.is_empty() {
                    self.entries.push(Entry::Assistant(text.clone()));
                }
                self.state = TuiState::Streaming;
            }
            SessionEvent::ToolCall {
                call_id,
                name,
                args,
            } => {
                self.entries.push(Entry::Tool(ToolRow {
                    call_id: call_id.clone(),
                    name: name.clone(),
                    target: args.clone(),
                    elapsed_ms: 0,
                    ok: false,
                    expanded: false,
                    detail: String::new(),
                }));
                self.state = TuiState::ToolRunning;
            }
            SessionEvent::Observation {
                call_id, output, ..
            } => {
                for entry in self.entries.iter_mut().rev() {
                    if let Entry::Tool(row) = entry {
                        if row.call_id == *call_id && !row.ok {
                            row.ok = true;
                            row.detail = output.clone();
                            break;
                        }
                    }
                }
                self.state = TuiState::Streaming;
            }
            SessionEvent::Done { summary } => {
                self.live = None;
                self.entries.push(Entry::Info(format!("done: {summary}")));
            }
            SessionEvent::Error { message } => {
                self.live = None;
                self.error = Some(message.clone());
                self.entries.push(Entry::Error(message.clone()));
                self.state = TuiState::Error;
            }
            SessionEvent::Reprompt { note } => {
                self.live = None;
                self.entries.push(Entry::Info(format!("reprompt: {note}")));
            }
            SessionEvent::Compaction { .. } => {
                self.entries
                    .push(Entry::Info("compacted earlier context".to_string()));
            }
            _ => {}
        }
        self.revision += 1;
    }

    pub fn show_approval(&mut self, command: &str, risk: &str, reversible: bool) {
        self.entries.push(Entry::Approval {
            command: command.to_string(),
            risk: risk.to_string(),
            reversible,
        });
        self.state = TuiState::Approval;
        self.revision += 1;
    }

    pub fn show_plan(&mut self, tasks: Vec<PlanTask>) {
        self.entries.push(Entry::Plan { tasks });
        self.state = TuiState::PlanGate;
        self.revision += 1;
    }

    pub fn set_error(&mut self, message: &str) {
        self.error = Some(message.to_string());
        self.state = TuiState::Error;
        self.revision += 1;
    }

    pub fn set_budget(&mut self, spent: u64, limit: u64) {
        self.budget = Some((spent, limit));
        self.state = TuiState::BudgetExhausted;
        self.revision += 1;
    }

    pub fn set_queued(&mut self, position: usize) {
        self.queued = Some(position);
        self.state = TuiState::Queued;
        self.revision += 1;
    }

    pub fn apply(&mut self, message: ServerMsg) {
        match message {
            ServerMsg::Attached { session_id } => self.session = session_id,
            ServerMsg::Event { event, .. } => self.push_event(&event),
            ServerMsg::Delta { text } => self.apply_delta(&text),
            ServerMsg::ApprovalRequest { id, tool, args, .. } => {
                self.pending_approval = Some(id);
                self.show_approval(&format!("{tool} {args}"), "ask", true);
            }
            ServerMsg::Busy { .. } => self.set_state(TuiState::Streaming),
            ServerMsg::Queued { position, .. } => self.set_queued(position),
            ServerMsg::Diff { .. } => {}
            ServerMsg::Dir { .. } => {}
            ServerMsg::File { .. } => {}
            ServerMsg::Artifacts { .. } => {}
            ServerMsg::Settings { .. } => {}
            ServerMsg::SelfView { text } => {
                self.self_text = Some(text);
                self.overlay = Some(Overlay::SelfState);
                self.revision += 1;
            }
            ServerMsg::Audit { .. } => {}
            ServerMsg::Sessions { sessions } => {
                self.entries
                    .push(Entry::Info(format!("{} sessions", sessions.len())));
                self.revision += 1;
            }
            ServerMsg::Error { message } => self.set_error(&message),
        }
    }

    pub fn apply_delta(&mut self, text: &str) {
        self.live.get_or_insert_with(String::new).push_str(text);
        self.state = TuiState::Streaming;
        self.revision += 1;
    }

    pub fn palette_commands(&self) -> Vec<(String, TuiAction)> {
        let all = vec![
            ("undo the last turn".to_string(), TuiAction::Undo),
            ("show self".to_string(), TuiAction::SelfView),
            ("interrupt the turn".to_string(), TuiAction::Interrupt),
            ("detach".to_string(), TuiAction::Detach),
            ("cancel".to_string(), TuiAction::Cancel),
        ];
        let query = self.palette_query.to_lowercase();
        all.into_iter()
            .filter(|(label, _)| query.is_empty() || label.contains(&query))
            .collect()
    }

    pub fn status_line(&self, width: u16) -> String {
        if width >= 100 {
            format!(
                "[{}] {} · worker {} · {} · {} · ctx {}% · {} · plan:{} · queue {}",
                self.state.label(),
                self.session,
                self.worker,
                self.model,
                self.provider,
                self.context_pct,
                self.cost,
                if self.plan_mode { "on" } else { "off" },
                self.queue
            )
        } else {
            format!(
                "[{}] {} · {} · {}%",
                self.state.label(),
                self.session,
                self.worker,
                self.context_pct
            )
        }
    }

    pub fn render(&self, frame: &mut Frame) {
        render::draw(frame, self);
    }
}
