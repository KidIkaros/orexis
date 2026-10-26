# M1-20 — Split the TUI modules

- **Blocked by:** —
- **Blocks:** —
- **Status:** resolved (devin, 2026-10-26)

## Goal

Split `adapters-tui/src/lib.rs` (507) and `render.rs` (453) into state, keys, render, and tests modules, each under 250 lines.

## Acceptance

- [x] `lib.rs` is module declarations and re-exports only (14 lines).
- [x] `state.rs` 126, `app.rs` 236, `keys.rs` 206, `render.rs` 192, `overlays.rs` 125, `client.rs` 111, `run.rs` 119.
- [x] Public API unchanged; all 26 tests stay green.
- [x] No behaviour change.

## Notes

- Tests live in `tests/app.rs`, `tests/client.rs`, and `tests/render.rs`.
- `render` stays public; `overlays` is `pub(crate)`.
- The revision cache and `apply` remain in `app.rs`.
