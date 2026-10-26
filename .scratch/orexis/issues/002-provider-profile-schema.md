# 002 — ProviderProfile schema and catalog source

- **Type:** grilling
- **Blocks:** M1
- **Status:** open
- **Claimed by:** —

## Question

What fields does `ProviderProfile` carry, and is the model catalog a vendored `models.dev` snapshot, a live fetch, or both? How do user profiles override shipped ones?

## Evidence

- Plan §4.4: profile = endpoint, auth kind, wire dialect, tool-call format, cache dialect, thinking support, context/pricing, model list.
- Hermes: 43 profiles under `plugins/model-providers/`, lazy discovery, last-writer-wins.
- opencode: `models.dev` catalog.
- Codex: server-delivered model catalog.

## Resolution

<filled when resolved>

## Follow-ups

<new tickets or fog this resolution creates>
