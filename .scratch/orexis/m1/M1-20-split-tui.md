# M1-20 — Split the TUI modules

- **Blocked by:** —
- **Blocks:** —
- **Status:** open

## Goal

Split `adapters-tui/src/lib.rs` (507 lines) and `render.rs` (453) into state, keys, render, and tests modules, each under 250 lines.

## Acceptance

- [ ] `lib.rs` is re-exports and module declarations only.
- [ ] `state.rs`, `keys.rs`, `render.rs`, and tests are separate.
- [ ] Public API unchanged; all 20 tests stay green.
- [ ] No behaviour change.
