# M8-03 — message-sqlite: the bus

- **Blocked by:** M8-01
- **Blocks:** M8-09
- **Status:** open

## Goal

The bus adapter (ADR-0020): direct (worker→worker) and room messages,
seq-ordered, persisted as events. `post` + `read(since)`; the room is
global to the team; direct messages address a worker id.

## Acceptance

- [ ] Post direct and room; read since a seq; the seq is monotonic.
- [ ] Messages are capped (unbounded tool output is an anti-pattern).
- [ ] Reopen preserves messages.
