# M8-08 — The supervisor: stall, restart, escalation

- **Blocked by:** M8-06, M8-07
- **Blocks:** M8-09
- **Status:** open

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
