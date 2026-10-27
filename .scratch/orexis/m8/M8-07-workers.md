# M8-07 — Workers: in-process turns over sessions, shared identity

- **Blocked by:** M8-05
- **Blocks:** M8-08
- **Status:** open

## Goal

A worker (ADR-0020, invariant 18): `{id, strategy, model, toolset, budget,
session_id}` — its execution is the existing `run_turn` over its own
session; the entity's identity context is injected on the first turn; the
strategy shapes prompt and toolset; the budget caps steps and wall clock.
Workers are tokio tasks in the daemon; never processes, never remote.

## Acceptance

- [ ] A worker runs a turn to done on its own session; the transcript is
      its session log.
- [ ] The identity context is shared (the entity's traits appear in the
      worker's first turn).
- [ ] The strategy's toolset filter narrows the gate.
- [ ] Budget exhaustion fails the task visibly.
