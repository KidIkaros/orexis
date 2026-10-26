# M7-10 — A2A client

- **Blocked by:** M7-01
- **Blocks:** M7-11
- **Status:** open

## Goal

Client-only A2A (ADR-0017 §6): agent card discovery, `message/send`,
`tasks/get`, SSE streaming where the peer supports it.

## Acceptance

- [ ] Agent card fetch and validation; unsupported capabilities degrade
      visibly.
- [ ] `message/send` and `tasks/get` over the HTTP shell; task state polled
      or streamed.
- [ ] Conformance fixture with a recorded card + task transcript.
- [ ] Remote agent output is untrusted content: delimited and capped.
- [ ] Manual: one real A2A sample agent completes a task.

## Notes

An A2A server is fog until team mode proves a need.
