# M2-01 — Tauri shell and daemon bridge

- **Blocked by:** 006
- **Blocks:** M2-02 … M2-06
- **Status:** open

## Goal

A single Tauri window with the three-zone skeleton and a Rust-side bridge to the daemon.

## Acceptance

- [ ] Tauri + Leptos workspace builds; one window, no web surface.
- [ ] Rust bridge attaches over the Unix socket, sends commands, emits `orexis://event`.
- [ ] Three zones render with placeholder panels; status bar and composer.
- [ ] No HTTP server; the webview makes no network calls.
- [ ] A scripted daemon event renders in the transcript.
