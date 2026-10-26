# M1-07 — Daemon and attach

- **Blocked by:** M1-01, M1-02, M1-06
- **Blocks:** M1-08
- **Status:** resolved (devin, 2026-10-26) — protocol/registry/replay; restart and second-client tests move to M1-08

## Goal

`orexis serve` owns sessions over a Unix-socket NDJSON protocol; clients attach, prompt, detach, and reattach; one local generation runs at a time.

## Acceptance

- [x] `ClientMsg`/`ServerMsg` round-trip over the socket.
- [x] Attach replays stored events by `seq`, then follows live; dedupe by `seq`.
- [x] Detach leaves the session running; reattach resumes without duplication.
- [x] Concurrent sessions queue behind the local provider's single generation slot (global semaphore).
- [ ] `kill -9` the daemon; a new daemon resumes from the last acknowledged event. *(M1-08 gate.)*
- [ ] A second client attaches to the same session read-only. *(M1-08 gate.)*
- [x] The daemon owns all session state; the TUI holds none. *(The TUI daemon client lands in M1-16.)*

## Notes

- `orexis-daemon`: `proto`, `session` (registry + broadcast sink), `server` (listener + connection loop).
- `EventSink` now carries the store's `seq`, so replay and live events dedupe correctly.
- Interrupt aborts the turn task; real approvals are M3 (the protocol messages exist, `Approve` returns a clear error).
- The `serve` binary is a skeleton with fakes; M1-08 wires the real catalog and tools.
