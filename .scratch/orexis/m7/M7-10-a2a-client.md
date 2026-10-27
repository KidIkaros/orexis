# M7-10 — A2A client

- **Blocked by:** M7-01
- **Blocks:** M7-11
- **Status:** resolved
- **Claimed by:** devin, 2026-10-26

## Goal

Client-only A2A (ADR-0017 §6): agent card discovery, `message/send`,
`tasks/get`, SSE streaming where the peer supports it.

## Acceptance

- [x] Agent card fetch and validation; unsupported capabilities degrade
      visibly.
- [x] `message/send` and `tasks/get` over the HTTP shell; task state polled
      or streamed.
- [x] Conformance fixture with a recorded card + task transcript.
- [x] Remote agent output is untrusted content: delimited and capped.
- [ ] Manual: one real A2A sample agent completes a task. (moved to M7-11 gate)

## Landed

- `adapters-protocol-a2a` (new crate): sans-io core (`core.rs`) — card
  parsing (v1.0 `supportedInterfaces` and v0.3 top-level `url`), task
  states with unknown-state degradation, `message/send`/`tasks/get`/
  `message/stream` request builders, result reading (task, direct
  message, or inline task). HTTP shell (`http.rs`) — card discovery
  (404 is its own `NoCard` error), method naming picked from the card's
  protocol version, SSE streaming through the shared decoder.
- Tool surface (`tools.rs`): `a2a/<agent>/{send,task}` through
  `ExternalTools` + `ToolExecutor`; output delimited in `<a2a_result>`
  with breakout escaping, capped at 64KB, descriptions capped at 500
  chars; peer configs load from user + project dirs, project wins.
- `is_external_tool` recognizes `a2a/` (plan mode denies, untrusted).
- `messageId` from a monotonic counter in the core — a same-length
  message must not repeat an id a peer could refuse as a replay
  (regression test: two same-length sends carry distinct ids).
- Composition: `a2a_dirs` config, `a2a_source` wiring; daemon env passes
  it through.
- Tests: 12 client fixtures + 7 tool-surface fixtures, all against
  recorded transcripts (no live peer).

## Notes

An A2A server is fog until team mode proves a need.
