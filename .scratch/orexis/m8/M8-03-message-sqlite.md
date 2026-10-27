# M8-03 — message-sqlite: the bus

- **Blocked by:** M8-01
- **Blocks:** M8-09
- **Status:** resolved
- **Claimed by:** devin, 2026-10-27

## Goal

The bus adapter (ADR-0020): direct (worker→worker) and room messages,
seq-ordered, persisted as events. `post` + `read(since)`; the room is
global to the team; direct messages address a worker id.

## Acceptance

- [ ] Post direct and room; read since a seq; the seq is monotonic.
- [ ] Messages are capped (unbounded tool output is an anti-pattern).
- [ ] Reopen preserves messages.

## Landed

- `adapters-message-sqlite` (new): `SqliteBus` — messages with a
  monotonic rowid seq; `read_since(seq, for_worker)` filters room +
  direct-to-worker in SQL; text capped at 64KB; WAL + synchronous FULL;
  reopen preserves.
- Tests: seq + filtering, the cap, reopen.
