# M2-04 — Fleet and board

- **Blocked by:** M2-01
- **Blocks:** M2-06
- **Status:** resolved (devin, 2026-10-26) — session projections per ADR-0012; Worker/Task metadata is M8

## Goal

Inspect workers and tasks without leaving the session.

## Acceptance

- [x] Session-projection table renders session, role, model, state, activity, cost. *(Model/activity/cost show —; real Worker metadata is M8 per ADR-0012.)*
- [x] Session-projection board renders session, stage, owner, blocked. *(Real Board tasks with owner/strategy/blocking edges are M8 per ADR-0012.)*
- [x] Selecting a worker/task opens that session through `attach`.
- [x] `1`–`5` switch rail panels; rows are tabbable; empty and connecting states.
