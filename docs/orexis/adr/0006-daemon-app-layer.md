# ADR-0006 — The daemon is an app layer

- **Status:** accepted
- **Date:** 2026-10-26
- **Deciders:** project owner, Devin
- **Ticket:** `.scratch/orexis/m1/M1-19-lint-hardening.md`

## Context

`orexis-daemon` owns the Unix socket, the session registry, and the attach protocol. It depends on `orexis-composition`, adapters, and usecases. It is not a provider/tool/store adapter, but it is also not the composition root: it must not construct SQLite stores itself.

## Decision

`orexis-daemon` is an **app/transport layer**:

- It may depend on `core`, `ports`, `usecases`, `composition`, and adapters.
- Nothing inward (`core`, `ports`, `usecases`, adapters) may depend on it.
- It constructs no adapters. Sessions arrive through the `SessionFactory` port, implemented by `SqliteSessionFactory` and constructed in composition.
- It may own process concerns: the listener, connection tasks, the generation semaphore, and the broadcast bus.

## Consequences

- The daemon is testable with an injected factory.
- The Dependency Rule still holds: the daemon is an outer ring.
- Import-lint treats it as an app, not an adapter.
- Folding it into the `orexis` binary later is a packaging choice, not an architecture change.

## Alternatives considered

- **Treat the daemon as an adapter** — rejected: it would sit inside the adapter ring while depending on composition.
- **Fold it into the binary now** — rejected: it would make the protocol untestable without a process.
- **Let the daemon construct the store** — rejected: two composition roots.
