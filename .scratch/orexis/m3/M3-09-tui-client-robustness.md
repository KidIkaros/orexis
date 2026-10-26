# M3-09 — TUI client robustness

- **Blocked by:** —
- **Blocks:** M2
- **Status:** in progress

## Goal

Never lose a partial NDJSON line, and surface a daemon disconnect instead of drawing forever.

## Acceptance

- [x] `DaemonClient` keeps a persistent byte buffer; a message split across reads is reassembled.
- [x] EOF returns `UnexpectedEof`, not `None`.
- [x] The TUI sets `Disconnected` and waits for a key before exiting.
- [x] Tests: split message, EOF.
