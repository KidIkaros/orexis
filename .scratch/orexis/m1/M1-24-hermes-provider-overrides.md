# M1-24 — Hermes provider overrides

- **Blocked by:** M1-23
- **Blocks:** —
- **Status:** resolved (devin, 2026-10-26)

## Goal

Use Hermes's provider profiles (MIT) as a curated source for base URLs, auth env/headers, aliases, and quirks, especially where models.dev has `api: null`.

## Acceptance

- [x] Extract provider data (name, base_url, api_mode, auth_type, env_vars, aliases, vision) without importing Python behavior at runtime.
- [x] Snapshot: 50 providers from `hermes-agent@bf0203a794`, MIT license.
- [x] Load only `chat_completions` / `anthropic_messages` with `api_key` or `none` auth and a non-empty base URL.
- [x] Skip OAuth, responses, Bedrock, Vertex, Azure placeholders, and empty base URLs.
- [x] Curated majors: OpenAI and Anthropic (existing), plus Groq, Mistral, and Gemini's OpenAI-compatible endpoint.
- [x] Provenance `hermes@bf0203a794` and `verified: false`; no Hermes code vendored.
- [x] Tests: DeepSeek base/models, MiniMax Anthropic kind, curated Anthropic auth, Groq/Mistral/Gemini base URLs.

## Notes

- Precedence is models.dev < Hermes < curated YAML; curated auth/wire wins, models are merged.
- Hermes-derived Anthropic-compatible providers (`minimax`, `commandcode`) assume `x-api-key`; their auth quirks are unverified.
- Azure, Bedrock, and Vertex need native adapters; they are intentionally skipped rather than mislabeled as OpenAI-compatible.
- `scripts/import-hermes-providers.py` regenerates the snapshot from a Hermes checkout.
