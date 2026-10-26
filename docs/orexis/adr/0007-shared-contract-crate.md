# ADR-0007 — Shared contract crate

- **Status:** accepted
- **Date:** 2026-10-26
- **Deciders:** project owner, Devin
- **Ticket:** `.scratch/orexis/m3/M3-12-adr-protocol.md`

## Context

The daemon and the TUI both speak the NDJSON protocol. The types cannot live in `orexis-ports` (they are transport messages, not consumer-side port traits) and cannot live in an adapter (the daemon and TUI would depend on each other). Duplicating them guarantees drift.

## Decision

`orexis-protocol` is a **shared contract crate**:

- It contains pure data types only (`ClientMsg`, `ServerMsg`, `SessionInfo`), with serde.
- It depends only on `orexis-core`.
- It may be depended on by apps and adapters (daemon, TUI, a future desktop client).
- It contains no I/O, no framing, and no policy.

## Consequences

- One wire contract; the daemon and every client serialize the same types.
- The Dependency Rule still holds: `protocol` is a sibling of `ports`, not an inner layer.
- Import-lint allows `adapters-*`/apps to depend on `orexis-protocol`; it does not allow `core`/`ports`/`usecases` to depend on adapters.
- Framing stays in the transport (`daemon::proto` NDJSON helpers, `DaemonClient`).

## Alternatives considered

- **Put the types in `ports`** — rejected: ports are consumer-side traits, not wire messages.
- **Put them in an adapter** — rejected: it would force a daemon/TUI dependency.
- **Duplicate the types** — rejected: drift.
