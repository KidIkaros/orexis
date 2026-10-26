# M3-04 — Fuzzy edit cascade

- **Blocked by:** M3-01
- **Blocks:** M3-08
- **Status:** resolved (devin, 2026-10-26)

## Goal

Handle edit drift without line numbers: exact match, then whitespace/Unicode canonicalization, then a bounded Levenshtein cascade. The tool, not the model, repairs the match.

## Acceptance

- [x] Exact match wins; no fuzzy repair when it is unique.
- [x] Whitespace and Unicode (NFC) variants match.
- [x] Levenshtein threshold (0.65) repairs near-matches.
- [x] A repair is reported in the observation (`exact`, `whitespace-normalized`, `fuzzy 0.92`).
- [x] Ambiguity and no-match still fail loudly.
- [x] Seven fixture tests cover exact, duplicate, whitespace, Unicode, fuzzy, no-match, ambiguous.

## Notes

- `edit::apply` returns the replaced text and the strategy used.
- Line-window matching keeps byte ranges correct; the trailing newline is preserved when `old` does not include one.
- Fuzzy matching is capped at 200 lines; larger blocks fall back to the whitespace strategy or fail.
