# ADR-0012 — M2 oversight projections vs the M8 team domain

- **Status:** accepted
- **Date:** 2026-10-26
- **Deciders:** project owner, Devin
- **Related:** CONTEXT.md, ADR-0008, M2-04, M2-05

## Context

`CONTEXT.md` defines **Worker** (an execution instance with its own context,
model, toolset, budget), **Board** (durable tasks with owner, strategy, and
blocking edges), and **Blackboard** (shared workspace, artifacts, leases) as
team concepts. M2 shipped fleet, board, and artifacts panels by projecting
**sessions** into those shapes: a session id as worker/task, session status as
stage, and changed files plus a checkpoint diff as artifacts.

That is useful oversight, but it is not the team domain. Using the same words
makes a session look like a worker and a status look like a task.

## Decision

1. **M2 fleet, board, and artifacts are session projections.** They derive from
   sessions and changes; they do not claim Worker, Task, or Artifact semantics.
2. **The UI labels them honestly:** `board*`, `fleet*`, and `changes`, with a
   caption that workers land in M8.
3. **The real Worker / Board / Artifact / Blackboard domain lands in M8**, with
   owner, strategy, blocking edges, and a durable artifact store.
4. **The M2 gate keeps six panels**, but fleet/board/changes are projections.
5. **`CONTEXT.md` gains** `Session projection`, `Fleet`, `Artifact`, and
   `Approval override` so the words are defined before use.

## Consequences

- M2 stays small and honest; it does not invent a team model it cannot back.
- M8 replaces projections with real domain objects; the UI contract can stay.
- The M2-04 acceptance is reworded from worker/task metadata to session
  projection; real metadata moves to M8.
- Artifacts are "changes" in M2; session-log artifacts wait for a real store.

## Alternatives considered

- **Build Worker/Task/Artifact stores now** — rejected: milestone-sized, and
  the team domain needs decisions the plan places in M8.
- **Keep the labels and call them placeholders** — rejected: the glossary
  would still be wrong, and users would read "worker" as a worker.
- **Remove the panels from M2** — rejected: session oversight is useful, and
  the six-panel layout is accepted in ADR-0008.
