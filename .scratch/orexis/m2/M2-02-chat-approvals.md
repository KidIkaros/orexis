# M2-02 — Chat, streaming, and approvals

- **Blocked by:** M2-01
- **Blocks:** M2-06
- **Status:** in progress (devin, 2026-10-26) — UI wired; Tauri shell unbuilt; y/n/d/a shortcuts and auto-open preview remain

## Goal

The primary task surface: transcript, streaming, composer, and the inline approval card.

## Acceptance

- [x] Streaming deltas render without blocking input (plumbing in M2-07; UI wiring here).
- [x] Tool rows collapse and expand; large output opens the right panel via an explicit "open in panel" action. *(Automatic threshold open remains.)*
- [ ] Approval card shows command, risk, reversibility; approve/deny buttons work. `y/n/d/a` shortcuts and "always" remain.
- [x] Approval never steals focus; a badge announces it; composer keeps autofocus.
- [x] Empty, running, error, budget, disconnected, queued, compacting states (status chip + banner).
- [x] A disconnect shows `disconnected` and offers a reattach button.
