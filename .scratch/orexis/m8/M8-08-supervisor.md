# M8-08 — The supervisor: stall, restart, escalation

- **Blocked by:** M8-06, M8-07
- **Blocks:** M8-09
- **Status:** resolved
- **Claimed by:** devin, 2026-10-27

## Goal

The supervisor (ADR-0020): a watchdog over workers — stall detection (no
session event within the budget's wall clock), restart (re-spawn the
worker on its own session; crash-recovery reuse), escalation (after N
restarts the task fails and the bus is told). The supervisor reads the
board and the bus; it does not run turns.

## Acceptance

- [ ] A stalled worker is detected and restarted on its session.
- [ ] N restarts escalate: the task fails, the bus is told.
- [ ] The supervisor never runs turns itself.

## Landed

- `usecases/src/supervisor.rs` (new): `SupervisorState` (restart counts
  per task; settled tasks clear theirs), `supervise()` — one tick reads
  the board, finds running tasks with no live worker, and plans the
  response: restart within the budget (the task stays running), or fail
  the task and tell the bus after `MAX_RESTARTS` (2). The supervisor
  never runs turns; the team loop (M8-09) executes the plan.
- Tests with test-local fakes: a stalled task is restarted within the
  budget; a live task is left alone; exhausting the budget escalates,
  fails the task with the reason on the board, tells the bus once, and
  clears the count; a recovered task's count resets so a later stall
  starts from zero.
