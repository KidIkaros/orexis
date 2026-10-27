# M8-07 — Workers: in-process turns over sessions, shared identity

- **Blocked by:** M8-05
- **Blocks:** M8-08
- **Status:** resolved
- **Claimed by:** devin, 2026-10-27

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

## Landed

- `daemon/src/team.rs` (new, pub): `WorkerTurn` (prompt suffix, toolset,
  max_steps, wall clock), `WorkerOutcome` (done, steps, error),
  `spawn_worker_turn` — the existing loop over the worker's own session,
  shaped by the strategy; the turn runs under the strategy's wall clock
  (a timeout is the outcome, not a hang); the handle returns the
  outcome for the team loop (M8-09) to turn into board updates.
- The turn assembly (identity on the first turn, the tool chain, the
  untrusted-content note, the context tools) was extracted from
  `spawn_turn` into `assemble_turn` — shared by session turns and worker
  turns, not duplicated; a worker's toolset narrows the gate's verdicts
  before they are asked (StrategyTools outside GatedTools).
- The daemon lib exports the worker API + `SessionRegistry`/
  `SessionRuntime`/`ApprovalHub`/`MemoryOverrides` for the tests and the
  coming team loop.
- Tests (`daemon/tests/worker.rs`): a worker runs to done on its own
  session; the entity's traits appear in the worker's first turn; the
  plan strategy's toolset refuses write_file with the narrowing error;
  the step budget exhausts visibly (steps counted, not an error); the
  wall clock exhausts visibly (the timeout is the error).
