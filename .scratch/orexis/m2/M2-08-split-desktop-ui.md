# M2-08 — Split the desktop UI

- **Blocked by:** —
- **Blocks:** —
- **Status:** open

## Goal

`crates/desktop/ui/src/main.rs` is 659 lines: state DTOs, Tauri IPC, diff rendering, shortcuts, and the whole app view. Split it before it grows further.

## Acceptance

- [ ] `state.rs` — Snapshot/Entry/Approval/SessionInfo DTOs.
- [ ] `ipc.rs` — invoke/listen/args.
- [ ] `diff.rs` — diff classification and unified/split rendering.
- [ ] `shortcuts.rs` — keyboard handling.
- [ ] `components/` — status, left rail, transcript, composer, right panel.
- [ ] No file over 250 lines; wasm check stays green.

## Notes

- The DTO mirrors `desktop-core::AppState`; a serde shape test guards drift.
- This is a clean-code finding from the M2-02/03 audit, not a behaviour change.
