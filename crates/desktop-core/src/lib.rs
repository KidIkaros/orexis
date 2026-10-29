//! The desktop state model and daemon bridge, independent of Tauri.

mod bridge;
pub mod diff;
mod state;

pub use bridge::DesktopBridge;
pub use diff::{
    align_split_diff, classify_diff_line, should_auto_expand_diff, DiffCell, DiffLineKind, SplitRow,
};
pub use state::{AppState, Approval, Connection, DesktopEntry, LeftTab, RightTab, Status};
