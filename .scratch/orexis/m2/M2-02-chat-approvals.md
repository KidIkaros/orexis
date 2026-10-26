# M2-02 — Chat, streaming, and approvals

- **Blocked by:** M2-01
- **Blocks:** M2-06
- **Status:** open

## Goal

The primary task surface: transcript, streaming, composer, and the inline approval card.

## Acceptance

- [ ] Streaming deltas render without blocking input.
- [ ] Tool rows collapse and expand; large output opens the right panel.
- [ ] Approval card shows command, risk, reversibility; `y/n/d/a` work.
- [ ] Approval never steals focus; a badge announces it.
- [ ] Empty, running, error, budget, disconnected, queued, compacting states.
- [ ] A disconnect shows `Disconnected` and offers reattach (the bridge currently stays dead until restart).
