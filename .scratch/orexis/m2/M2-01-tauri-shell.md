# M2-01 — Tauri shell and daemon bridge

- **Blocked by:** 006
- **Blocks:** M2-02 … M2-06
- **Status:** in progress (bridge/state complete; shell build blocked on system libraries)

## Goal

A single Tauri window with the three-zone skeleton and a Rust-side bridge to the daemon.

## Acceptance

- [ ] Tauri + Leptos workspace builds; one window, no web surface. *(Leptos UI wasm check passes; the Tauri side needs `webkit2gtk-4.1`, `gtk+-3.0`, `trunk`, and `tauri-cli`, which are absent here.)*
- [x] Rust bridge attaches over the Unix socket, sends commands, emits `orexis://event` (source + desktop-core tests).
- [x] Three zones render with placeholder panels; status bar and composer (Leptos source).
- [x] No HTTP server; the webview makes no network calls (design).
- [x] A scripted daemon event renders in the transcript (state test).

## Notes

- `orexis-client` now owns the shared `DaemonClient`; the TUI re-exports it.
- `orexis-desktop-core` owns `AppState`, `DesktopEntry`, and `DesktopBridge`; six tests cover apply, tool pairing, approval, and disconnect.
- The Tauri app lives in `crates/desktop`, excluded from the workspace because Tauri's system libraries are not in CI. It contains the Rust commands, the pump thread, `tauri.conf.json`, and a Leptos three-zone shell.
- `cargo check --manifest-path crates/desktop/ui/Cargo.toml --target wasm32-unknown-unknown` passes (Leptos 0.8.22).
- Run instructions and prerequisites are in `crates/desktop/README.md`.
- The shell must build on a machine with webkit/gtk before this ticket closes.
