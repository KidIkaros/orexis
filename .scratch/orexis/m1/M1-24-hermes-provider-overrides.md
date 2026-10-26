# M1-24 — Hermes provider overrides

- **Blocked by:** M1-23
- **Blocks:** —
- **Status:** open

## Goal

Use Hermes's 51 provider profiles (MIT) as a curated source for base URLs, auth env/headers, aliases, and quirks, especially for providers where models.dev has `api: null`.

## Acceptance

- [ ] Extract provider data (name, base_url, env_vars, api_mode, auth_type, aliases, headers) without importing Python behavior.
- [ ] Add curated overrides for the majors: OpenAI, Anthropic, Groq, Mistral, Google, Azure, Bedrock.
- [ ] Map `api_mode` to our `kind`/wire; mark native-only wires that need adapters.
- [ ] Provenance (`hermes@<commit>`, MIT) on extracted profiles.
- [ ] Tests: majors resolve a base URL; curated verified profiles still win.
- [ ] No Hermes code is vendored; data only.

## Notes

- Hermes profiles are `ProviderProfile(...)` calls under `plugins/model-providers/<name>/__init__.py`; static extraction is preferred over executing Hermes.
- models.dev remains the model-metadata source; Hermes supplies provider plumbing.
