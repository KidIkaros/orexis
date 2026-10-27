# M8-06 — The scheduler: admission under capacity, roster, depth

- **Blocked by:** M8-02, M8-05
- **Blocks:** M8-08
- **Status:** open

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
