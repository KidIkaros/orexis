# M1-16 — TUI interactive loop and polish

- **Blocked by:** M1-07
- **Blocks:** —
- **Status:** open

## Goal

The deferred half of ADR-0004: the real crossterm event loop, paste and external editor, IME cursor anchoring, command palette and search, and transcript line caching.

## Acceptance

- [ ] A crossterm event loop drives `handle_key` and redraws without blocking on provider output.
- [ ] Bracketed paste inserts text; `Ctrl-E` opens `$EDITOR` and returns the result.
- [ ] The composer tracks the IME cursor anchor.
- [ ] `Ctrl-P` opens a filterable command palette; `Ctrl-R` searches the transcript.
- [ ] Completed transcript blocks are cached; only the live tail re-renders.
- [ ] A 10k-entry transcript renders at 60 fps in a `TestBackend` benchmark.
- [ ] No new panes; ADR-0004 remains the boundary.

## Notes

- The render/keymap/state contract shipped in M1-06; this ticket is the interactive shell and the performance work.
- The event loop needs the daemon client from M1-07, hence the blocker.
