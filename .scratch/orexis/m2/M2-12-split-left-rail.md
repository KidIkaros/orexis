# M2-12 — Split the left rail again

- **Blocked by:** —
- **Blocks:** —
- **Status:** resolved (devin, 2026-10-26)

## Goal

M2-08 left `left.rs` at 51 lines; M2-04/M2-05 grew it to 314.

## Acceptance

- [x] `left/mod.rs` 93 (tab shell + shared helpers), `views.rs` 108 (sessions/board/fleet), `files.rs` 133 (changes/files).
- [x] No file over 250; wasm check green.
- [x] UI labels use ADR-0012 projection language.

## Notes

- Board and fleet carry a "session projection · workers land in M8" caption.
- The changes tab is no longer called artifacts.
