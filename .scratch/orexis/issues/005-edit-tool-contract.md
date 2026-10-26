# 005 — Edit-tool contract per model tier

- **Type:** prototype
- **Blocks:** M3
- **Status:** open
- **Claimed by:** —

## Question

What is the edit contract per model tier — exact unique-substring replacement for frontier models, fuzzy cascade for weaker ones — and how is drift handled without line numbers?

## Evidence

- Corpus §16.4: exact unique-substring for frontier; fuzzy cascade (Levenshtein ~0.65) for weaker; never line numbers.
- opencode: nine-stage cascade.
- Hermes: nine-strategy chain.
- Harness: `edit_file` anchored replace.

## Resolution

<filled when resolved>

## Follow-ups

<new tickets or fog this resolution creates>
