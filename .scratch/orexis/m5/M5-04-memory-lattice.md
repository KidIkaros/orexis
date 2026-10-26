# M5-04 — Memory lattice and lexical recall

- **Blocked by:** M5-01
- **Blocks:** M5-05, M5-06
- **Status:** open

## Goal

Bounded lexical memory across episodic, semantic, procedural, and identity kinds.

## Acceptance

- [ ] Memory records with kind, text, tags, source seq, confidence, timestamps.
- [ ] Writes and forgets append `MemoryWrite`/`MemoryForget`.
- [ ] Recall scores by tags/keywords, recency, and confidence; returns bounded top-k.
- [ ] No embeddings, no vector store, no code RAG.
- [ ] Tests: recall ranking, forgetting, bounded output, export.

## Notes

- Procedural memory is how-to knowledge, not executable code.
