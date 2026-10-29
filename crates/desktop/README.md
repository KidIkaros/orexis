# Orexis desktop (Tauri + Leptos)

This app is excluded from the main workspace because Tauri needs system
libraries that are not present in CI. The testable state model and daemon
bridge live in `crates/desktop-core` and are covered by the workspace tests.

## Prerequisites

- Linux: `libwebkit2gtk-4.1-dev`, `libgtk-3-dev`,
  `libayatana-appindicator3-dev`, `librsvg2-dev`.
- Rust target `wasm32-unknown-unknown`.
- `cargo install trunk tauri-cli`.

## Run

```bash
# terminal 1: daemon (fake provider for a UI smoke test)
OREXIS_FAKE=1 OREXIS_SOCKET=/tmp/orexis.sock OREXIS_DB=/tmp/orexis.db   cargo run -p orexis-daemon --bin serve

# terminal 2: UI
cd crates/desktop/ui && trunk serve
# terminal 3: shell
cd crates/desktop/src-tauri && cargo tauri dev
```

The shell attaches with `invoke('attach', { socket, session: null })`; the
Rust bridge emits `orexis://event` for each `ServerMsg`.

## Install from Release

```bash
curl -fsSL https://raw.githubusercontent.com/KidIkaros/orexis/main/scripts/install.sh | bash -s -- --desktop
```

## Status

Three zones, status bar, transcript listener, composer, left/right panels,
hunk-aligned split diff viewer, and automatic full-screen diff expansion.
Packaged and signed in CI via `scripts/release.sh desktop <target>` (`.github/workflows/release.yml`).
