# M1-02 — Provider catalog + OpenAI-compatible adapter (ADR-0002)

- **Blocked by:** —
- **Blocks:** M1-03, M1-04, M1-05, M1-06, M1-07, M1-08
- **Status:** in progress (devin, 2026-10-26) — core + keyring complete; live endpoint run pending

## Goal

Implement the ADR-0002 profile schema, the vendored catalog, precedence resolution, keyring/env secrets, and the OpenAI-compatible adapter with streaming and native tool calls.

## Acceptance

- [x] A profile parses from YAML with the rich core + `extra`.
- [x] Precedence resolves shipped < user < project < env/CLI; `source_of` reports the winner. (`providers show` CLI lands with M1-07.)
- [x] The catalog loads offline from the vendored snapshot (openai, anthropic, llamacpp).
- [x] Env, memory, and OS keyring secret stores work (`secret-tool` backend; live smoke test passed).
- [x] OpenAI-compatible `chat/completions` streams deltas and tool calls against a mock transport.
- [ ] One real OpenAI-compatible endpoint completes a scripted tool-call task (manual; needs credentials).
- [x] Unknown models get conservative defaults and a warning.

## Remaining

- One live endpoint run (manual; needs credentials).
- `providers show` CLI (M1-07).
- Native macOS Keychain / Windows Credential Manager backends behind the same `KeyringBackend` seam.

## Notes

- The adapter depends on a `Transport` trait, not `reqwest`; SSE parsing is tested without a network.
- `Provider::complete` now carries `&[ToolSpec]`; native tool calls need the schema.
- The keyring uses the freedesktop `secret-tool` CLI because the `keyring` crate's Secret Service backend needs libdbus headers, which are not installed here. The `KeyringBackend` seam allows a native adapter later; the smoke test stored, looked up, and cleared a throwaway entry.
- Composition resolves the keyring first, then environment variables.
- Request hashes and redaction are already in the session store (M1-01).
