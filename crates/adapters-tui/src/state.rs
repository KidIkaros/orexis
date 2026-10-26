//! TUI state types.

use ratatui::text::Line;

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
    Palette,
    Search,
}

impl Overlay {
    pub fn title(self) -> &'static str {
        match self {
            Self::Fleet => "fleet",
            Self::Board => "board",
            Self::SelfState => "self",
            Self::Help => "help",
            Self::Palette => "palette",
            Self::Search => "search",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ToolRow {
    pub call_id: String,
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
    Undo,
    SelfView,
    EditExternal,
}

pub struct TranscriptCache {
    pub revision: u64,
    pub lines: Vec<Line<'static>>,
}
