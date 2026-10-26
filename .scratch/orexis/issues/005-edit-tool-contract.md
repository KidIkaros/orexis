# 005 — Edit-tool contract per model tier

- **Type:** prototype
- **Blocks:** M3
- **Status:** resolved
- **Claimed by:** —

## Question

What is the edit contract per model tier — exact unique-substring replacement for frontier models, fuzzy cascade for weaker ones — and how is drift handled without line numbers?

## Evidence

- Corpus §16.4: exact unique-substring for frontier; fuzzy cascade (Levenshtein ~0.65) for weaker; never line numbers.
- opencode: nine-stage cascade.
- Hermes: nine-strategy chain.
- Harness: `edit_file` anchored replace.

## Resolution

Exact unique-substring replacement first. If it does not match, compare
line windows after whitespace collapse and Unicode NFC, then a bounded
Levenshtein window (>= 0.65, at most 200 lines). Ambiguity at any stage fails
loudly; line numbers are never used. The trailing newline is preserved when
`old` omits it, and the observation reports which strategy repaired the match.
Implemented in M3-04.

## Follow-ups

None.
