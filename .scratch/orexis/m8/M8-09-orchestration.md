# M8-09 — The team orchestration loop + daemon wiring + real panels

- **Blocked by:** M8-03, M8-04, M8-08
- **Blocks:** M8-10
- **Status:** resolved
- **Claimed by:** devin, 2026-10-27

## Goal

The team loop (ADR-0020): board → scheduler → workers → bus/blackboard,
composed in the daemon (ADR-0006); opt-in via `OREXIS_TEAM=1`. Worker
output consumed by other workers is delimited and capped (invariant 22).
The panels and the model's board tools are M8-11 — this ticket is the
loop and its wiring.

## Acceptance

- [ ] Team mode off: today's single-agent behavior, unchanged.
- [ ] Team mode on: tasks flow board → workers → done.
- [ ] The entity sees worker outcomes as session outcomes.
- [ ] Composition constructs the adapters (rule 3); the daemon wires the
      loop.

## Landed

- `composition`: constructs `SqliteBoard`, `SqliteBus`, `FsBlackboard`
  (rule 3) — paths derived from the db path and the project root;
  `Runtime` gains `board`/`bus`/`blackboard`; the fake runtime wires the
  fakes.
- `daemon/src/team.rs`: `TeamLoop` — a 250ms tick of reap → watch →
  admit → start-admitted. Reap: finished worker turns become board
  updates, bus messages, and entity outcomes (one entity, many workers,
  invariant 18). Watch: the supervisor's plan executed — stalled running
  tasks re-spawned on their own session (the task → session map is the
  loop's memory, reused across restarts). Admit: `admit_ready` under the
  provider's capacity and the roster. Start-admitted: a task becomes a
  running worker when a generation slot is free (`try_acquire` — a task
  left admitted is started on a later tick, never blocking the loop).
  The loop's maps are tokio mutexes (guards held across awaits).
- `Daemon::run` spawns the loop only when `config.team` — opt-in
  (invariant 19); `OREXIS_TEAM=1` sets it; `DaemonConfig` is Clone.
- Tests (`daemon/tests/team_loop.rs`): team on — two independent tasks
  flow board → workers → done, the bus carries the completions, the
  entity recorded the outcomes; team off — the board is untouched, the
  single-agent behavior unchanged.
- The panels and the model's board tools are M8-11 (split out of this
  ticket).
