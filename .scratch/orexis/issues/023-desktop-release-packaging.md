# 023 — Desktop workspace release packaging (v0.2.0, M2)

- **Type:** task
- **Blocks:** v0.2.0
- **Status:** open
- **Claimed by:** —

## Question

How should the Orexis desktop application (`crates/desktop` Tauri v2 + Leptos frontend) be packaged, signed, and integrated with the v0.1.0 release pipeline for multi-platform distribution on Linux and macOS?

## Evidence

- `crates/desktop/src-tauri/tauri.conf.json`: Tauri v2 configuration configured with bundle targets.
- `crates/desktop/ui`: Leptos 0.7 WebAssembly frontend with six oversight panels (chat, fleet, approvals, diffs, artifacts, files).
- `crates/desktop-core`: Pure Rust bridge crate connecting to the daemon socket.
- ADR-0008: Desktop three-zone oversight workspace.
- ADR-0010: Shared outer layer for desktop and client crates.
- ADR-0019: Minisign signing key is shared with the Tauri updater.
- `scripts/m2-gate.sh`: Automated desktop test gate.

## Proposed Resolution

1. **Release packaging in `.github/workflows/release.yml`**:
   - Add desktop packaging jobs building Tauri bundles on `ubuntu-latest` (`.deb`, AppImage) and `macos-latest` (`.dmg`).
   - Sign bundles using the existing minisign key (`OREXIS_SIGNING_KEY`).
2. **Frontend ergonomics and oversight**:
   - Implement side-by-side / unified diff view with syntax highlighting in Leptos (`crates/desktop/ui/src/components/diff.rs`).
   - Wire hunk-level approval controls calling `approve_with`.
3. **Usability evaluations**:
   - Execute evaluation cycle 1 (cognitive walkthrough of 5 oversight scenarios).
   - Execute evaluation cycle 2 (interactive responsiveness: p95 keystroke-to-render < 16ms).

## Follow-ups

- Publish `v0.2.0` release assets on GitHub with desktop installers.
- Document desktop installation in `README.md`.
