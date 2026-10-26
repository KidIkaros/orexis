# M2-04 — Fleet and board

- **Blocked by:** M2-01
- **Blocks:** M2-06
- **Status:** in progress (devin, 2026-10-26) — session-derived panels; richer worker/task metadata lands with multi-agent

## Goal

Inspect workers and tasks without leaving the session.

## Acceptance

- [x] Fleet table renders worker, role, model, state, task, cost. *(Role is "session"; model/task/cost show — until the daemon exposes them.)*
- [x] Board renders task id, title, stage, worker, blocked-by. *(Derived from sessions; blocked-by shows — until task metadata exists.)*
- [x] Selecting a worker/task opens that session through `attach`.
- [x] `1`–`5` switch rail panels; rows are tabbable; empty and connecting states.
