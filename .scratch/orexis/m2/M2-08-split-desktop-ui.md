# M2-08 — Split the desktop UI

- **Blocked by:** —
- **Blocks:** —
- **Status:** resolved (devin, 2026-10-26)

## Goal

`crates/desktop/ui/src/main.rs` is 659 lines: state DTOs, Tauri IPC, diff rendering, shortcuts, and the whole app view. Split it before it grows further.

## Acceptance

- [x] `state.rs` — Snapshot/Entry/Approval/SessionInfo DTOs (85).
- [x] `ipc.rs` — invoke/listen/args (41).
- [x] `diff.rs` — diff classification and unified/split rendering (51).
- [x] `shortcuts.rs` — keyboard handling (63).
- [x] `components/` — status 57, left 51, transcript 101, composer 44, right 126.
- [x] No file over 250 lines; `app.rs` 160, `main.rs` 13; wasm check green.

## Notes

- Line-count regression after M2-04/M2-05 (`left.rs` 314) was closed by M2-12.
- The DTO mirrors `desktop-core::AppState`; a serde shape test guards drift.
- This is a clean-code finding from the M2-02/03 audit, not a behaviour change.
