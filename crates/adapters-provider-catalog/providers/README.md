# Provider snapshots

- `models-dev.json` — vendored from <https://models.dev/api.json>, fetched
  2026-10-26, MIT (`models-dev.LICENSE`). It supplies long-tail provider base
  URLs and model metadata (context, output, pricing, tools, vision, reasoning).
- Curated `*.yaml` profiles override generated entries by id. Model metadata is
  merged into curated profiles, so curated auth/wire/quirks win while the model
  list stays broad.
- Generated entries are marked `verified: false` and carry a `provenance` field.
- Regenerate with `scripts/fetch-models-dev.sh`; the catalog works offline.

Known gaps: the major providers (OpenAI, Anthropic, Groq, Mistral, Google,
Azure, Bedrock) have `api: null` in models.dev. Their base URLs and auth quirks
come from curated profiles or Hermes extraction (M1-24).

## Hermes provider data

- `hermes-providers.json` — data extracted from `hermes-agent@bf0203a794`
  with `scripts/import-hermes-providers.py`, MIT (`hermes.LICENSE`). No Hermes
  code is vendored.
- 50 providers extracted; **34 loaded**. 16 skipped: 4 empty/placeholder base URLs, 5 responses/Bedrock, 6 OAuth/copilot/external/vertex, 1 hidden.
- Precedence: models.dev < Hermes < curated `*.yaml`.
- Curated majors added here: `groq`, `mistral`, and `gemini` (OpenAI-compatible
  endpoint). OpenAI and Anthropic already have curated profiles.
- Hermes-derived Anthropic-compatible providers (`minimax`, `commandcode`)
  assume `x-api-key`; their auth quirks are unverified until tested.
