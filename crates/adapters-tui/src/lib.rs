//! The TUI state machine, keymap, and renderer (ADR-0004).

mod app;
mod keys;
pub(crate) mod overlays;
pub mod render;
mod run;
mod state;

pub use app::TuiApp;
pub use orexis_client::DaemonClient;
pub use run::run;
pub use state::{Entry, Focus, Overlay, PlanTask, ToolRow, TranscriptCache, TuiAction, TuiState};
