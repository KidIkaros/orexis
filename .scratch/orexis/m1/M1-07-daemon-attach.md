# M1-07 — Daemon and attach

- **Blocked by:** M1-01, M1-02, M1-06
- **Blocks:** M1-08
- **Status:** open

## Goal

`orexis serve` owns sessions over a Unix-socket NDJSON protocol; clients attach, prompt, detach, and reattach; one local generation runs at a time.

## Acceptance

- [ ] `ClientMsg`/`ServerMsg` round-trip over the socket.
- [ ] Attach replays stored events by `seq`, then follows live; dedupe by `seq`.
- [ ] Detach leaves the session running; reattach resumes without duplication.
- [ ] Concurrent sessions queue behind the local provider's single generation slot.
- [ ] `kill -9` the daemon; a new daemon resumes from the last acknowledged event.
- [ ] A second client attaches to the same session read-only.
- [ ] The TUI is a thin client: no session state outside the daemon.

## Notes

- Reuse Harness's `proto.rs` shape; the transport is a detail.
- Approvals and interrupts bypass the session input queue (they must not deadlock a busy loop).
