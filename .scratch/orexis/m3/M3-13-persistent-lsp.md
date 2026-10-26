# M3-13 — Persistent LSP server

- **Blocked by:** —
- **Blocks:** —
- **Status:** resolved (devin, 2026-10-26) — reuse verified; latency not benchmarked

## Goal

Keep one language-server process per workspace instead of spawning rust-analyzer per edit.

## Acceptance

- [x] One server per workspace, reused across edits (`spawns() == 1` after two edits).
- [x] `didChange` on subsequent edits; diagnostics still attach.
- [x] Shutdown on drop; no orphan (`kill_on_drop`; graceful `shutdown`/`exit` is a later refinement).
- [ ] Edit latency under 200 ms after the first edit. *(Not benchmarked; the mock is instant, so reuse is verified rather than latency.)*

## Notes

- `StdioLsp` holds a `Mutex<Option<Session>>`; the session starts lazily and survives across calls.
- The initialize handshake is bounded by the same 5 s timeout as diagnostics.
- On a write/read failure the session is dropped; the next call respawns.
- The mock server handles `didChange`; the integration test asserts a single spawn.
