# M5-04 — Memory lattice and lexical recall

- **Blocked by:** M5-01
- **Blocks:** M5-05, M5-06
- **Status:** resolved (devin, 2026-10-26)

## Goal

Bounded lexical memory across episodic, semantic, procedural, and identity kinds.

## Acceptance

- [x] Memory records with kind, text, tags, source seq, confidence. *(No entity timestamp yet; recency uses source seq.)*
- [x] `remember`/`forget` append `MemoryWrite`/`MemoryForget`.
- [x] Recall scores exact tags > text > tag substrings, plus confidence/recency; bounded 1–50.
- [x] Lexical only; no embedding or vector dependency.
- [x] Tests: tag/text ranking, kind filter, limit, empty query, write/forget.

## Notes

- Procedural memory is how-to knowledge, not executable code.
- Recall is a pure function over the entity snapshot; no turn-loop wiring yet (M5-06).
- Recency uses `source_seq`; a dedicated entity timestamp is a later refinement.
