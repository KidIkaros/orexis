# M2-07 — Streaming deltas end to end

- **Blocked by:** —
- **Blocks:** M2-02
- **Status:** resolved (devin, 2026-10-26)

## Goal

Token deltas must cross the ports, the loop, the daemon, and both clients. The providers previously collected SSE and returned only the final response.

## Acceptance

- [x] `DeltaSink` port with a `NullDeltaSink`; `Provider::stream` default emits final text as one delta.
- [x] OpenAI-compatible and Anthropic adapters emit each content delta.
- [x] The turn loop calls `provider.stream` with the sink.
- [x] The daemon broadcasts `ServerMsg::Delta` per session.
- [x] The TUI and desktop keep a live buffer; the final `ModelResponse` clears it and commits the entry.
- [x] Tests: adapter deltas, TUI live buffer, desktop live buffer, daemon forwarding.

## Notes

- Only text deltas are surfaced; tool-call partial arguments remain internal.
- Deltas are ephemeral: they are not persisted or replayed.
- The default `stream` keeps fake/test providers working with one delta.
