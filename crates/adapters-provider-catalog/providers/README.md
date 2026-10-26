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
