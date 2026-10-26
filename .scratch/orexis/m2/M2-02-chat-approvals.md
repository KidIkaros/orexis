# M2-02 — Chat, streaming, and approvals

- **Blocked by:** M2-01
- **Blocks:** M2-06
- **Status:** in progress (devin, 2026-10-26) — UI wired; Tauri shell unbuilt; approval exit transition remains

## Goal

The primary task surface: transcript, streaming, composer, and the inline approval card.

## Acceptance

- [x] Streaming deltas render without blocking input (plumbing in M2-07; UI wiring here).
- [x] Tool rows collapse and expand; output over 2 KB opens automatically in the right panel, and "open in panel" works for any row.
- [x] Approval card shows command, risk, reversibility; `y/n/d/a` shortcuts and approve/deny/always/diff buttons work. *(d is a diff preview until M2-03; always is daemon-wide, not per-session.)*
- [x] Approval never steals focus; a badge announces it; composer keeps autofocus.
- [x] Empty, running, error, budget, disconnected, queued, compacting states (status chip + banner).
- [x] A disconnect shows `disconnected` and offers a reattach button.

## Notes

- Approval shortcuts ignore keypresses while typing in the composer.
- "Always" adds the tool to a daemon-wide allow set; per-session scoping is a later refinement.
- `d` shows the pending command in the right panel; a real diff lands with M2-03.
- The approval card enters with a subtle rise; exit is still instant.
