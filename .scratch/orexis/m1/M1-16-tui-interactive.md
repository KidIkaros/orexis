# M1-16 — TUI interactive loop and polish

- **Blocked by:** M1-07
- **Blocks:** M3-08
- **Status:** resolved (devin, 2026-10-26)

## Goal

The deferred half of ADR-0004: the real crossterm event loop, paste and external editor, IME cursor anchoring, command palette and search, and transcript line caching.

## Acceptance

- [x] A crossterm event loop drives `handle_key` and redraws without blocking on provider output (30 ms poll + non-blocking `try_recv`).
- [x] Bracketed paste inserts text; `Ctrl-E` opens `$EDITOR` and returns the result.
- [x] The composer tracks the cursor; the renderer sets the terminal cursor position.
- [x] `Ctrl-P` opens a filterable command palette; `Ctrl-R` searches the transcript.
- [x] Completed transcript blocks are cached; only a new event or an expand invalidates the cache.
- [ ] A 10k-entry transcript renders at 60 fps in a `TestBackend` benchmark. *(Not measured; performance work is M1-20.)*
- [x] No new panes; ADR-0004 remains the boundary.

## Notes

- `orexis-protocol` now owns `ClientMsg`/`ServerMsg`; the daemon re-exports them.
- `DaemonClient` is synchronous: 50 ms read timeout, 5 s attach deadline, NDJSON.
- `run(socket, session)` sets up raw mode/alternate screen, attaches, and loops.
- Palette commands: undo, interrupt, detach, cancel; search filters transcript lines.
- 24 TUI tests cover the client round-trip, apply, palette, search, states, and cache invalidation.
