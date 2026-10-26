# M3-13 — Persistent LSP server

- **Blocked by:** —
- **Blocks:** —
- **Status:** open

## Goal

Keep one language-server process per workspace instead of spawning rust-analyzer per edit.

## Acceptance

- [ ] One server per workspace, reused across edits.
- [ ] didChange on subsequent edits; diagnostics still attach.
- [ ] Shutdown on drop; no orphan.
- [ ] Edit latency under 200 ms after the first edit.
