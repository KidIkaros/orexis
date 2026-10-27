# ADR-0020 — The team-mode domain: workers over sessions, three stores, a capacity-bounded scheduler

- **Status:** accepted
- **Date:** 2026-10-27
- **Deciders:** project owner, Devin
- **Related:** PLAN §9 (team mode), §4.1 (crates), invariants 16–19, ADR-0012 (M2 projections), ADR-0006 (daemon app layer), study: Kindred (orchestration, planner→executor→verifier), DeepSeek (team limits), synthesis (serial default)

## Context

M8 is the last unspecced milestone: workers, strategies, board, bus,
blackboard, scheduler. The plan decided the shape long ago (§9): the entity
is singular; workers are generic execution instances sharing its identity;
strategies are use cases, never agent types; concurrency is provider-bounded;
single-agent is the default and team mode is opt-in, gated by a benchmark
(≥15% pass rate or ≥25% wall-clock at ≤2× tokens — the 15× prior to beat).

Corpus evidence:

- **8/9 harnesses keep sub-agents in-process**; multi-agent costs ~15×
  tokens (the prior to beat).
- **DeepSeek**: bounded teams (`maxMembers` 16, `maxTasks` 256), versioned
  task snapshots, expected-revision writes, explicit write scopes;
  concurrency across processes explicitly unsupported.
- **Kindred**: planner → executor → verifier with `Clarify` instead of
  guess-calling; orchestration gates fold into the permission ruleset, not a
  second policy system.
- **ADR-0012** promised M8 replaces M2's session projections with the real
  Worker/Board/Artifact domain, keeping the UI contract.

## Decision

### Workers are in-process turns over sessions

A **worker** is `{ id, strategy, model, toolset, budget, session_id }`. Its
execution is the existing `run_turn` loop over its own session log — no new
execution engine, no second loop. What makes it a worker: the board task it
owns, the strategy shaping its prompt and toolset, its own model and budget,
and its visibility in the fleet. The entity's identity context is injected
into a worker's first turn exactly as the daemon does for sessions
(invariant 18: workers share identity and memory; only context, model,
toolset, and budget are per-worker). Workers are tokio tasks in the daemon —
never processes, never remote (spawn depth 1, flat).

### Three stores, three ports, three adapters

| Port | Aggregate | Adapter | Notes |
|---|---|---|---|
| `BoardStore` | tasks: id, title, required strategy, owner, status, `depends_on` edges, seq | `board-sqlite` | statuses: `queued → admitted → running → blocked → done / failed`; edges are task ids that must be `done` |
| `MessageStore` | the bus: direct (worker→worker) and room messages, seq-ordered | `message-sqlite` | persisted events, not ephemeral chat (CONTEXT.md) |
| `ArtifactStore` | durable outputs + leases on workspace paths | `artifacts-fs` | a lease names an owner and expires; artifacts are files under the blackboard root |

`message-sqlite` is a new adapter crate beyond §4.1's list — recorded here as
the plan-body change the spec makes. Board and bus are separate aggregates
with independent change rates; one combined adapter would be the
zone-of-pain magnet the architecture forbids.

### The scheduler asks the provider for capacity

`Provider` gains `fn capacity(&self) -> usize` (default 1; adapters return
the profile's `max_concurrency`) — invariant 17 made real instead of the
daemon's semaphore guess. Admission: a queued task whose `depends_on` are
all `done`, whose required strategy has a strategy definition, admitted up
to `min(provider capacity, roster ≤ 16)`. Exhaustion produces a visible
queue (the board's `queued` rows), never a silent drop.

### Strategies are use-case definitions

A strategy is data + a use case: `{ id: plan/build/review/mine, prompt
fragment, toolset filter, verify policy, budget defaults }`. The `build`
strategy carries Kindred's planner → executor → verifier shape with
`Clarify` instead of guess-calling. A strategy never names an agent type
(invariant 16); orchestration gates fold into the permission ruleset — no
second policy system.

### The supervisor is a watchdog, not the loop

Stall detection (no session event for a worker within its budget's wall
clock), restart (re-spawn the worker on its own session — crash recovery
reuse), escalation (after N restarts: the task fails, the bus is told). The
supervisor reads the board and the bus; it does not run turns.

### Opt-in, gated

Team mode is off by default; `OREXIS_TEAM=1` (or the settings surface)
enables it. The M8 gate is the benchmark comparison (§10): team beats serial
on a breadth task at ≤2× tokens, or team mode stays experimental and the
cut rule applies. The bench subcommand's breadth tasks are part of M8.

## Consequences

- One turn loop serves single-agent and team modes; the team layer composes
  it, it does not duplicate it.
- The M2 fleet/board/changes panels read the real domain when it exists
  (ADR-0012's contract), falling back to projections when team mode is off.
- Three new adapter crates (`board-sqlite`, `message-sqlite`,
  `artifacts-fs`) and four new ports; the import-lint and seam graph carry
  the additions.
- The benchmark gate can reject the whole milestone's headline feature —
  that is the point of the gate.

## Alternatives considered

- **Workers as processes** (isolation, crash safety) — rejected: the corpus
  keeps sub-agents in-process (8/9), process-per-worker multiplies state
  and startup cost, and crash recovery already exists at the session layer.
- **One combined team-sqlite adapter** — rejected: board and bus are
  separate aggregates; combining them couples their schemas and change
  rates.
- **Strategies as agent types with their own loops** — rejected: invariant
  16; a second loop is a second architecture to keep correct.
- **Scheduler as a queue alone** — rejected: §9 names admission under
  provider capacity and budget as the scheduler's job; a queue without
  capacity is a silent overload.

## Follow-ups

The M8 spec (`docs/orexis/specs/M8.md`) and its tickets carry the build
order: ports → adapters → strategies → scheduler → workers → supervisor →
orchestration → the benchmark gate.
