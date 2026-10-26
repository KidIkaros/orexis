//! The TUI state machine and keymap (ADR-0004). Rendering lives in `render`.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use orexis_core::SessionEvent;
use ratatui::Frame;

pub mod render;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TuiState {
    Empty,
    Loading,
    Streaming,
    ToolRunning,
    Approval,
    PlanGate,
    Error,
    BudgetExhausted,
    Disconnected,
    Queued,
    Compacting,
}

impl TuiState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Loading => "loading",
            Self::Streaming => "streaming",
            Self::ToolRunning => "tool",
            Self::Approval => "approval",
            Self::PlanGate => "plan",
            Self::Error => "error",
            Self::BudgetExhausted => "budget",
            Self::Disconnected => "detached",
            Self::Queued => "queued",
            Self::Compacting => "compacting",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Composer,
    Transcript,
    Overlay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overlay {
    Fleet,
    Board,
    SelfState,
    Help,
}

impl Overlay {
    pub fn title(self) -> &'static str {
        match self {
            Self::Fleet => "fleet",
            Self::Board => "board",
            Self::SelfState => "self",
            Self::Help => "help",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ToolRow {
    pub name: String,
    pub target: String,
    pub elapsed_ms: u64,
    pub ok: bool,
    pub expanded: bool,
    pub detail: String,
}

#[derive(Debug, Clone)]
pub struct PlanTask {
    pub text: String,
    pub done: bool,
    pub current: bool,
}

#[derive(Debug, Clone)]
pub enum Entry {
    User(String),
    Assistant(String),
    Tool(ToolRow),
    Approval {
        command: String,
        risk: String,
        reversible: bool,
    },
    Plan {
        tasks: Vec<PlanTask>,
    },
    Error(String),
    Info(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TuiAction {
    None,
    Send(String),
    Cancel,
    Interrupt,
    Detach,
    Quit,
    Approve,
    Deny,
    Diff,
    AlwaysAllow,
    PlanApprove,
    PlanEdit,
    PlanCancel,
    OpenFile(String),
    Copy,
}

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
        }
    }
}

impl TuiApp {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_state(&mut self, state: TuiState) {
        self.state = state;
    }

    pub fn push_event(&mut self, event: &SessionEvent) {
        match event {
            SessionEvent::UserMessage { text } => {
                self.entries.push(Entry::User(text.clone()));
                self.state = TuiState::Streaming;
            }
            SessionEvent::ModelResponse { text, .. } if !text.is_empty() => {
                self.entries.push(Entry::Assistant(text.clone()));
                self.state = TuiState::Streaming;
            }
            SessionEvent::ToolCall { name, args, .. } => {
                self.entries.push(Entry::Tool(ToolRow {
                    name: name.clone(),
                    target: args.clone(),
                    elapsed_ms: 0,
                    ok: false,
                    expanded: false,
                    detail: String::new(),
                }));
                self.state = TuiState::ToolRunning;
            }
            SessionEvent::Observation { name, output, .. } => {
                for entry in self.entries.iter_mut().rev() {
                    if let Entry::Tool(row) = entry {
                        if row.name == *name && !row.ok {
                            row.ok = true;
                            row.detail = output.clone();
                            break;
                        }
                    }
                }
                self.state = TuiState::Streaming;
            }
            SessionEvent::Done { summary } => {
                self.entries.push(Entry::Info(format!("done: {summary}")));
            }
            SessionEvent::Error { message } => {
                self.error = Some(message.clone());
                self.entries.push(Entry::Error(message.clone()));
                self.state = TuiState::Error;
            }
            SessionEvent::Reprompt { note } => {
                self.entries.push(Entry::Info(format!("reprompt: {note}")));
            }
            SessionEvent::Compaction { .. } => {
                self.entries
                    .push(Entry::Info("compacted earlier context".to_string()));
            }
            _ => {}
        }
    }

    pub fn show_approval(&mut self, command: &str, risk: &str, reversible: bool) {
        self.entries.push(Entry::Approval {
            command: command.to_string(),
            risk: risk.to_string(),
            reversible,
        });
        self.state = TuiState::Approval;
    }

    pub fn show_plan(&mut self, tasks: Vec<PlanTask>) {
        self.entries.push(Entry::Plan { tasks });
        self.state = TuiState::PlanGate;
    }

    pub fn set_error(&mut self, message: &str) {
        self.error = Some(message.to_string());
        self.state = TuiState::Error;
    }

    pub fn set_budget(&mut self, spent: u64, limit: u64) {
        self.budget = Some((spent, limit));
        self.state = TuiState::BudgetExhausted;
    }

    pub fn set_queued(&mut self, position: usize) {
        self.queued = Some(position);
        self.state = TuiState::Queued;
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> TuiAction {
        match self.state {
            TuiState::Approval => match key.code {
                KeyCode::Char('y') => TuiAction::Approve,
                KeyCode::Char('n') => TuiAction::Deny,
                KeyCode::Char('d') => TuiAction::Diff,
                KeyCode::Char('a') => TuiAction::AlwaysAllow,
                KeyCode::Esc => {
                    self.state = TuiState::Streaming;
                    TuiAction::Cancel
                }
                _ => TuiAction::None,
            },
            TuiState::PlanGate => match key.code {
                KeyCode::Char('a') => TuiAction::PlanApprove,
                KeyCode::Char('e') => TuiAction::PlanEdit,
                KeyCode::Char('c') => {
                    self.state = TuiState::Streaming;
                    TuiAction::PlanCancel
                }
                _ => TuiAction::None,
            },
            _ => self.handle_normal(key),
        }
    }

    fn handle_normal(&mut self, key: KeyEvent) -> TuiAction {
        if self.overlay.is_some() {
            if key.code == KeyCode::Esc {
                self.overlay = None;
                self.focus = Focus::Composer;
            }
            return TuiAction::None;
        }

        match key.code {
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                TuiAction::Interrupt
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                TuiAction::Detach
            }
            KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.overlay = Some(Overlay::Fleet);
                self.focus = Focus::Overlay;
                TuiAction::None
            }
            KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.overlay = Some(Overlay::Board);
                self.focus = Focus::Overlay;
                TuiAction::None
            }
            KeyCode::Char('?') => {
                self.overlay = Some(Overlay::Help);
                self.focus = Focus::Overlay;
                TuiAction::None
            }
            KeyCode::Tab => {
                self.focus = match self.focus {
                    Focus::Composer => Focus::Transcript,
                    _ => Focus::Composer,
                };
                TuiAction::None
            }
            KeyCode::Esc => TuiAction::Cancel,
            _ => match self.focus {
                Focus::Composer => self.handle_composer(key),
                Focus::Transcript => self.handle_transcript(key),
                Focus::Overlay => TuiAction::None,
            },
        }
    }

    fn handle_composer(&mut self, key: KeyEvent) -> TuiAction {
        match key.code {
            KeyCode::Enter if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.composer.push('\n');
                TuiAction::None
            }
            KeyCode::Enter => {
                let text = std::mem::take(&mut self.composer);
                TuiAction::Send(text)
            }
            KeyCode::Backspace => {
                self.composer.pop();
                TuiAction::None
            }
            KeyCode::Char(c) => {
                self.composer.push(c);
                TuiAction::None
            }
            _ => TuiAction::None,
        }
    }

    fn handle_transcript(&mut self, key: KeyEvent) -> TuiAction {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.scroll = self.scroll.saturating_add(1);
                TuiAction::None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.scroll = self.scroll.saturating_sub(1);
                TuiAction::None
            }
            KeyCode::Char('g') => {
                self.scroll = 0;
                TuiAction::None
            }
            KeyCode::Char('G') => {
                self.scroll = usize::MAX;
                TuiAction::None
            }
            KeyCode::Enter => {
                if let Some(Entry::Tool(row)) = self.entries.get_mut(self.selected) {
                    row.expanded = !row.expanded;
                    if row.expanded && row.detail.lines().count() > 8 {
                        self.bottom_sheet = Some(row.detail.clone());
                    }
                }
                TuiAction::None
            }
            KeyCode::Char('o') => {
                if let Some(Entry::Tool(row)) = self.entries.get(self.selected) {
                    return TuiAction::OpenFile(row.target.clone());
                }
                TuiAction::None
            }
            KeyCode::Char('y') => TuiAction::Copy,
            KeyCode::Char('q') => TuiAction::Quit,
            _ => TuiAction::None,
        }
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

    pub fn keyhints(&self) -> &'static str {
        match self.state {
            TuiState::Approval => "y approve · n deny · d diff · a always",
            TuiState::PlanGate => "a approve · e edit · c cancel",
            _ if self.overlay.is_some() => "Esc close · j/k navigate · Enter select",
            _ if self.focus == Focus::Transcript => {
                "j/k scroll · Enter expand · o open · y copy · Tab composer"
            }
            _ => "Enter send · ⇧Enter newline · Esc cancel · ^P palette · ^D detach · ? help",
        }
    }

    pub fn render(&self, frame: &mut Frame) {
        render::draw(frame, self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn enter_sends_the_composer() {
        let mut app = TuiApp::new();
        app.composer = "hello".to_string();
        assert_eq!(
            app.handle_key(key(KeyCode::Enter)),
            TuiAction::Send("hello".into())
        );
        assert!(app.composer.is_empty());
    }

    #[test]
    fn shift_enter_newlines() {
        let mut app = TuiApp::new();
        app.composer = "one".to_string();
        let event = KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT);
        assert_eq!(app.handle_key(event), TuiAction::None);
        assert_eq!(app.composer, "one\n");
    }

    #[test]
    fn esc_cancels() {
        let mut app = TuiApp::new();
        assert_eq!(app.handle_key(key(KeyCode::Esc)), TuiAction::Cancel);
    }

    #[test]
    fn approval_y_approves() {
        let mut app = TuiApp::new();
        app.show_approval("rm -rf target", "high", false);
        assert_eq!(app.handle_key(key(KeyCode::Char('y'))), TuiAction::Approve);
    }

    #[test]
    fn tab_toggles_focus() {
        let mut app = TuiApp::new();
        assert_eq!(app.focus, Focus::Composer);
        app.handle_key(key(KeyCode::Tab));
        assert_eq!(app.focus, Focus::Transcript);
        app.handle_key(key(KeyCode::Tab));
        assert_eq!(app.focus, Focus::Composer);
    }

    #[test]
    fn tool_observation_pairs_with_the_call() {
        let mut app = TuiApp::new();
        app.push_event(&SessionEvent::ToolCall {
            call_id: "c1".into(),
            name: "read".into(),
            args: "src/lib.rs".into(),
        });
        app.push_event(&SessionEvent::Observation {
            call_id: "c1".into(),
            name: "read".into(),
            output: "ok".into(),
        });
        let Entry::Tool(row) = &app.entries[0] else {
            panic!("expected tool row");
        };
        assert!(row.ok);
        assert_eq!(row.detail, "ok");
    }
}
