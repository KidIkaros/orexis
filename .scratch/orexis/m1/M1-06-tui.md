# M1-06 — ratatui TUI (ADR-0004)

- **Blocked by:** M1-01, M1-02, M1-05
- **Blocks:** M1-07, M1-08
- **Status:** resolved (devin, 2026-10-26) — render/keymap/state contract; interactive loop moved to M1-16

## Goal

Implement the ADR-0004 surface: four fixed regions, composer-first focus, inline tool expansion with a bottom sheet for large diffs, adaptive overlays and status bar, virtualized transcript, and the eleven states.

## Acceptance

- [x] Snapshot tests for all eleven states at 80×24 and 120×40.
- [x] Overlays draw over the transcript without reflow or scroll loss.
- [x] Composer: multiline (Shift+Enter), backspace, cursor position. *(Paste, external editor, and IME moved to M1-16.)*
- [x] Keymap matches ADR-0004 for send/cancel/interrupt/detach/focus/approval/plan/overlays. *(Palette and search moved to M1-16.)*
- [x] `NO_COLOR` respected; state is never colour-only.
- [x] Streaming deltas update the transcript via `push_event`. *(The crossterm event loop lands with M1-07.)*
- [x] Tool rows collapse to one line and expand inline; a diff over the viewport opens a bottom sheet.
- [x] Status bar adapts at 80 vs 120 columns.
- [x] Twenty tests, including all states at both sizes.

## Notes

- `orexis-adapters-tui`: `TuiApp` (state + keymap) and `render` (ratatui).
- Transcript rendering virtualizes the visible window; block caching is M1-16.
- The daemon client and real event loop are M1-07; the TUI is already event-shaped (`push_event`, `handle_key`).
