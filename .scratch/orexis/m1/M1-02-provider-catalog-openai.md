# M1-02 — Provider catalog + OpenAI-compatible adapter (ADR-0002)

- **Blocked by:** —
- **Blocks:** M1-03, M1-04, M1-05, M1-06, M1-07, M1-08
- **Status:** in progress (devin, 2026-10-25) — core complete; keyring + live run pending

## Goal

Implement the ADR-0002 profile schema, the vendored catalog, precedence resolution, keyring/env secrets, and the OpenAI-compatible adapter with streaming and native tool calls.

## Acceptance

- [x] A profile parses from YAML with the rich core + `extra`.
- [x] Precedence resolves shipped < user < project < env/CLI; `source_of` reports the winner. (`providers show` CLI lands with M1-07.)
- [x] The catalog loads offline from the vendored snapshot (openai, anthropic, llamacpp).
- [x] Env and memory secret stores work. **OS keyring adapter remains.**
- [x] OpenAI-compatible `chat/completions` streams deltas and tool calls against a mock transport.
- [ ] One real OpenAI-compatible endpoint completes a scripted tool-call task (manual; needs credentials).
- [x] Unknown models get conservative defaults and a warning.

## Remaining

- OS keyring adapter (Secret Service / Keychain / Credential Manager), behind a platform feature.
- `providers show` CLI (M1-07).
- One live endpoint run.

## Notes

- The adapter depends on a `Transport` trait, not `reqwest`; SSE parsing is tested without a network.
- `Provider::complete` now carries `&[ToolSpec]`; native tool calls need the schema.
- Request hashes and redaction are already in the session store (M1-01).
