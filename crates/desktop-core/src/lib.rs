//! The desktop state model and daemon bridge, independent of Tauri.

mod bridge;
mod state;

pub use bridge::DesktopBridge;
pub use state::{AppState, Approval, Connection, DesktopEntry, LeftTab, RightTab, Status};
