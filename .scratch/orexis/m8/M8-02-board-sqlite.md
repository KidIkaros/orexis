# M8-02 — board-sqlite: tasks, edges, statuses

- **Blocked by:** M8-01
- **Blocks:** M8-06
- **Status:** resolved
- **Claimed by:** devin, 2026-10-27

## Goal

The board adapter (ADR-0020): tasks `{id, title, required_strategy, owner,
status, depends_on, seq}`; statuses `queued → admitted → running → blocked
→ done / failed`; edges are task ids that must be `done`. The
session-sqlite pattern (schema version, migration adjacent).

## Acceptance

- [ ] Create, claim (admit → running with an owner), complete, fail,
      block/unblock by edges.
- [ ] `ready()` returns queued tasks whose edges are all done.
- [ ] The 256-task cap is enforced at create (visible refusal).
- [ ] Reopen preserves tasks (the crash-recovery property).

## Landed

- `adapters-board-sqlite` (new): `SqliteBoard` — tasks with edges in a
  `task_edge` table; `ready()` is one SQL join (queued + all edges done);
  transitions are guarded (`from` status checked in the UPDATE, errors
  name the actual state); the cap (256, `open_with_cap` to tighten)
  refuses visibly; WAL + synchronous FULL; reopen preserves tasks and
  their statuses.
- Tests: the fake's contracts replayed against the real store (edges,
  cap, ordered transitions, unknown edges/tasks, reopen).
