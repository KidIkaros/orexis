#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use orexis_desktop_core::DesktopBridge;
use tauri::{Emitter, Manager, State};

struct DesktopState {
    bridge: Mutex<Option<DesktopBridge>>,
}

impl DesktopState {
    fn with<T>(&self, f: impl FnOnce(&DesktopBridge) -> Result<T, String>) -> Result<T, String> {
        let guard = self
            .bridge
            .lock()
            .map_err(|_| "bridge lock poisoned".to_string())?;
        let bridge = guard.as_ref().ok_or_else(|| "not attached".to_string())?;
        f(bridge)
    }

    fn with_mut<T>(
        &self,
        f: impl FnOnce(&mut DesktopBridge) -> Result<T, String>,
    ) -> Result<T, String> {
        let mut guard = self
            .bridge
            .lock()
            .map_err(|_| "bridge lock poisoned".to_string())?;
        let bridge = guard.as_mut().ok_or_else(|| "not attached".to_string())?;
        f(bridge)
    }
}

fn emit_state(handle: &tauri::AppHandle) -> Result<(), String> {
    let state = handle.state::<DesktopState>();
    let snapshot = state
        .with(|bridge| serde_json::to_value(bridge.state()).map_err(|error| error.to_string()))?;
    handle
        .emit("orexis://state", &snapshot)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn attach(
    app: tauri::AppHandle,
    state: State<DesktopState>,
    socket: String,
    session: Option<String>,
) -> Result<(), String> {
    let bridge = DesktopBridge::connect(&PathBuf::from(socket), session)
        .map_err(|error| error.to_string())?;
    *state
        .bridge
        .lock()
        .map_err(|_| "bridge lock poisoned".to_string())? = Some(bridge);
    emit_state(&app)
}

#[tauri::command]
fn prompt(
    app: tauri::AppHandle,
    state: State<DesktopState>,
    text: String,
    skip_verify: bool,
) -> Result<(), String> {
    state.with_mut(|bridge| {
        bridge
            .prompt(&text, skip_verify)
            .map_err(|error| error.to_string())
    })?;
    emit_state(&app)
}

#[tauri::command]
fn approve(
    app: tauri::AppHandle,
    state: State<DesktopState>,
    id: u64,
    yes: bool,
    always: bool,
) -> Result<(), String> {
    state.with_mut(|bridge| {
        bridge
            .approve_with(id, yes, always)
            .map_err(|error| error.to_string())
    })?;
    emit_state(&app)
}

#[tauri::command]
fn self_view(app: tauri::AppHandle, state: State<DesktopState>) -> Result<(), String> {
    state.with_mut(|bridge| bridge.self_view().map_err(|error| error.to_string()))?;
    emit_state(&app)
}

#[tauri::command]
fn settings(app: tauri::AppHandle, state: State<DesktopState>) -> Result<(), String> {
    state.with_mut(|bridge| bridge.settings().map_err(|error| error.to_string()))?;
    emit_state(&app)
}

#[tauri::command]
fn list_dir(
    app: tauri::AppHandle,
    state: State<DesktopState>,
    path: String,
) -> Result<(), String> {
    state.with_mut(|bridge| bridge.list_dir(&path).map_err(|error| error.to_string()))?;
    emit_state(&app)
}

#[tauri::command]
fn read_file(
    app: tauri::AppHandle,
    state: State<DesktopState>,
    path: String,
) -> Result<(), String> {
    state.with_mut(|bridge| bridge.read_file(&path).map_err(|error| error.to_string()))?;
    emit_state(&app)
}

#[tauri::command]
fn artifacts(app: tauri::AppHandle, state: State<DesktopState>) -> Result<(), String> {
    state.with_mut(|bridge| bridge.artifacts().map_err(|error| error.to_string()))?;
    emit_state(&app)
}

#[tauri::command]
fn board(app: tauri::AppHandle, state: State<DesktopState>) -> Result<(), String> {
    state.with_mut(|bridge| bridge.board().map_err(|error| error.to_string()))?;
    emit_state(&app)
}

#[tauri::command]
fn fleet(app: tauri::AppHandle, state: State<DesktopState>) -> Result<(), String> {
    state.with_mut(|bridge| bridge.fleet().map_err(|error| error.to_string()))?;
    emit_state(&app)
}

#[tauri::command]
fn diff(app: tauri::AppHandle, state: State<DesktopState>) -> Result<(), String> {
    state.with_mut(|bridge| bridge.diff().map_err(|error| error.to_string()))?;
    emit_state(&app)
}

#[tauri::command]
fn undo(app: tauri::AppHandle, state: State<DesktopState>) -> Result<(), String> {
    state.with_mut(|bridge| bridge.undo().map_err(|error| error.to_string()))?;
    emit_state(&app)
}

#[tauri::command]
fn interrupt(app: tauri::AppHandle, state: State<DesktopState>) -> Result<(), String> {
    state.with_mut(|bridge| bridge.interrupt().map_err(|error| error.to_string()))?;
    emit_state(&app)
}

#[tauri::command]
fn list_sessions(app: tauri::AppHandle, state: State<DesktopState>) -> Result<(), String> {
    state.with_mut(|bridge| bridge.list_sessions().map_err(|error| error.to_string()))?;
    emit_state(&app)
}

#[tauri::command]
fn snapshot(state: State<DesktopState>) -> Result<serde_json::Value, String> {
    state.with(|bridge| serde_json::to_value(bridge.state()).map_err(|error| error.to_string()))
}

fn main() {
    tauri::Builder::default()
        .manage(DesktopState {
            bridge: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            attach,
            prompt,
            approve,
            diff,
            self_view,
            settings,
            list_dir,
            read_file,
            artifacts,
            board,
            fleet,
            undo,
            interrupt,
            list_sessions,
            snapshot
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            std::thread::spawn(move || loop {
                let messages = {
                    let state = handle.state::<DesktopState>();
                    state.with_mut(|bridge| bridge.pump().map_err(|error| error.to_string()))
                };
                if let Ok(messages) = messages {
                    let had_messages = !messages.is_empty();
                    for message in messages {
                        let _ = handle.emit("orexis://event", &message);
                    }
                    if had_messages {
                        let _ = emit_state(&handle);
                    }
                }
                std::thread::sleep(Duration::from_millis(50));
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Orexis");
}
