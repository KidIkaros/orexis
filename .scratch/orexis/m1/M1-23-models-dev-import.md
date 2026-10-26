# M1-23 — models.dev catalog import

- **Blocked by:** —
- **Blocks:** M1-24
- **Status:** resolved (devin, 2026-10-26)

## Goal

Vendor models.dev as the broad provider/model base without turning the catalog into 232 hand-written YAMLs.

## Acceptance

- [x] The MIT snapshot is vendored (`providers/models-dev.json`, 5.7 MB, fetched 2026-10-26) with its license.
- [x] Providers with an `api` base URL become profiles (207 of 232).
- [x] Model metadata (context, output, pricing, tools, vision, reasoning) is merged into curated profiles by provider id.
- [x] Curated YAML profiles override generated entries; auth/wire/quirks win.
- [x] Generated entries carry `provenance: models.dev@2026-10-26` and `verified: false`.
- [x] `scripts/fetch-models-dev.sh` refreshes the snapshot; runtime stays offline.
- [x] Tests: snapshot count, long-tail provider, metadata merge, curated auth.

## Notes

- models.dev's `api` is null for the majors (OpenAI, Anthropic, Groq, Mistral, Google, Azure, Bedrock). Those need curated or Hermes-derived base URLs (M1-24).
- Generated profiles default to OpenAI-compatible, native tools, SSE; native-only wires are unverified until an adapter or curated override exists.
- Snapshot parsing is ~0.17 s in tests; no runtime network.
