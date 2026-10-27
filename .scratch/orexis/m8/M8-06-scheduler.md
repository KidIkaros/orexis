# M8-06 — The scheduler: admission under capacity, roster, depth

- **Blocked by:** M8-02, M8-05
- **Blocks:** M8-08
- **Status:** resolved
- **Claimed by:** devin, 2026-10-27

## Goal

The scheduler (ADR-0020, invariant 17): asks the provider for capacity;
admits queued tasks whose edges are done, up to
`min(provider capacity, roster ≤ 16)`; exhaustion produces a visible
queue, never a silent drop. Spawn depth 1 (flat).

## Acceptance

- [ ] Admission respects capacity (asked from the provider, not guessed).
- [ ] Edges gate admission (a task with unfinished `depends_on` waits).
- [ ] The roster cap (16) and the 256-task board cap hold.
- [ ] Exhaustion is visible (the queue is readable).

## Landed

- `usecases/src/scheduler.rs` (new): `admit_ready(board, capacity, roster)`
  — slots = min(provider capacity, roster) minus running tasks; FIFO by
  board seq; the rest stay queued (the visible queue). `ROSTER_CAP = 16`.
- Tests with a test-local board fake (use cases depend on ports only —
  the adapters-fake crate is not a usecases dependency): capacity
  respected, the roster caps, running tasks occupy slots, zero capacity
  admits nothing (the local provider's single slot), edges gate.
- Found and fixed while wiring: `usecases/src/lib.rs` had never declared
  `mod strategy;` — the M8-05 module was committed as dead code and its
  tests never ran. Both modules are now wired; the strategy tests run.
