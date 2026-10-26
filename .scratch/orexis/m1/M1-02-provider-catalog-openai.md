# M1-02 — Provider catalog + OpenAI-compatible adapter (ADR-0002)

- **Blocked by:** —
- **Blocks:** M1-03, M1-04, M1-05, M1-06, M1-07, M1-08
- **Status:** open

## Goal

Implement the ADR-0002 profile schema, the vendored catalog, precedence resolution, keyring/env secrets, and the OpenAI-compatible adapter with streaming and native tool calls.

## Acceptance

- [ ] A profile parses from YAML with the rich core + `extra`.
- [ ] Precedence resolves shipped < user < project < env/CLI; `providers show` explains the winner.
- [ ] The catalog loads offline from the vendored snapshot.
- [ ] The keyring adapter stores and retrieves a key; env fallback works.
- [ ] OpenAI-compatible `chat/completions` streams deltas and tool calls against a mock server.
- [ ] One real OpenAI-compatible endpoint completes a scripted tool-call task.
- [ ] Unknown models get conservative defaults, a warning, and an override.

## Notes

- The adapter normalizes `tool_calls` into `ToolCall`; the codec sees one shape.
- Cache dialect and thinking parameters are profile-driven, not hard-coded.
