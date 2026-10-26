# M3-04 — Fuzzy edit cascade

- **Blocked by:** M3-01
- **Blocks:** M3-08
- **Status:** open

## Goal

Handle edit drift without line numbers: exact match, then whitespace/Unicode canonicalization, then a bounded Levenshtein cascade. The tool, not the model, repairs the match.

## Acceptance

- [ ] Exact match wins; no fuzzy repair when it is unique.
- [ ] Whitespace and Unicode variants match.
- [ ] Levenshtein threshold (0.65) repairs near-matches.
- [ ] A repair is reported in the observation.
- [ ] Ambiguity and no-match still fail loudly.
- [ ] Fixture suite for drift cases.
