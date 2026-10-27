# M8 — Team mode

Spec: `docs/orexis/specs/M8.md`; decisions: ADR-0020.

| Ticket | Goal | Blocked by |
|---|---|---|
| [M8-01](M8-01-ports.md) | Ports: BoardStore, MessageStore, ArtifactStore, Provider::capacity | — |
| [M8-02](M8-02-board-sqlite.md) | board-sqlite: tasks, edges, statuses | M8-01 | — resolved
| [M8-03](M8-03-message-sqlite.md) | message-sqlite: the bus | M8-01 | — resolved
| [M8-04](M8-04-artifacts-fs.md) | artifacts-fs: artifacts and leases | M8-01 | — resolved
| [M8-05](M8-05-strategies.md) | Strategies as use cases | M8-01 | — resolved
| [M8-06](M8-06-scheduler.md) | The scheduler: admission | M8-02, M8-05 | — resolved
| [M8-07](M8-07-workers.md) | Workers: in-process turns over sessions | M8-05 | — resolved
| [M8-08](M8-08-supervisor.md) | The supervisor: stall, restart, escalation | M8-06, M8-07 |
| [M8-09](M8-09-orchestration.md) | The team loop + daemon wiring + real panels | M8-03, M8-04, M8-08 |
| [M8-10](M8-10-bench-gate.md) | Bench breadth tasks + the team-vs-serial gate | M8-09 |
