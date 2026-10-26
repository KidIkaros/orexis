# 002 — ProviderProfile schema and catalog source

- **Type:** grilling
- **Blocks:** M1
- **Status:** resolved
- **Claimed by:** devin, 2026-10-25

## Question

What fields does `ProviderProfile` carry, and is the model catalog a vendored `models.dev` snapshot, a live fetch, or both? How do user profiles override shipped ones?

## Evidence

- Plan §4.4: profile = endpoint, auth kind, wire dialect, tool-call format, cache dialect, thinking support, context/pricing, model list.
- Hermes: 43 profiles under `plugins/model-providers/`, lazy discovery, last-writer-wins.
- opencode: `models.dev` catalog.
- Codex: server-delivered model catalog.

## Resolution

**Decided 2026-10-25.** See `docs/orexis/adr/0002-provider-catalog.md` for the full record.

- **Profile shape:** rich typed core plus an `extra` map for quirks. Core: identity, auth, wire dialect, tool format, stream format, cache dialect, thinking, capabilities, models, limits, privacy, fallbacks.
- **Catalog:** a pinned snapshot generated from models.dev plus curated profiles, shipped in-tree; `orexis providers refresh` updates it; fully offline by default.
- **Precedence:** shipped < user (`~/.config/orexis/providers/`) < project (`.orexis/providers/`) < env/CLI. `orexis providers show` explains the winner.
- **Secrets:** OS keyring by default (Secret Service / Keychain / Credential Manager), environment fallback, never plaintext config.
- **Unknown models:** conservative dialect defaults, a visible warning, and a one-line override.
- **Capacity:** the profile declares `max_concurrency`; local defaults to 1, remote to 4; overridable.
- **Fallbacks:** per-profile ordered chain; triggers on auth failure, 429 after retries, or repeated 5xx; per-session override.
- **OAuth:** adapter-owned device flow; tokens in the keyring; refresh on 401; never logged.

**Shape (YAML):**

```yaml
id: anthropic
display_name: Anthropic
kind: anthropic
base_url: https://api.anthropic.com
auth: { kind: api_key, env: [ANTHROPIC_API_KEY], header: x-api-key }
wire: { dialect: anthropic, tool_format: native, stream_format: sse }
cache: { dialect: anthropic, breakpoints: true }
thinking: { supported: true, param: thinking, levels: [low, medium, high] }
capabilities: { tools: true, parallel_tools: true, vision: true }
models:
  - { id: claude-sonnet-4-5, context: 200000, max_output: 64000, pricing: { input: 3.0, output: 15.0 } }
limits: { max_concurrency: 4 }
fallbacks: [openai]
extra: {}
```

## Follow-ups

- ADR-0002 written.
- M1 implements the schema, the vendored catalog, precedence resolution, and the keyring adapter.
- Ticket 009 (native adapter timing) consumes this schema.
