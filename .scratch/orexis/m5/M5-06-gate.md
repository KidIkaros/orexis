# M5-06 — M5 gate

- **Blocked by:** M5-01 … M5-05
- **Blocks:** —
- **Status:** resolved (devin, 2026-10-26) — automated; manual `/self` and live model switch remain

## Goal

Prove the entity persists, stays evidence-backed, and never leaks mid-session.

## Acceptance

- [x] `scripts/m5-gate.sh`: M4 gate + entity/drive/affect/memory/identity suites.
- [x] Mid-session trait added between turns; the second system prompt contains neither old nor new identity.
- [x] Restart and shared-handle persistence tests. *(Live model switch is manual.)*
- [x] Entity JSONL export/import round-trip.
- [x] Evidence-required and decay fixtures.
- [x] Recall capped 1–50; gate greps for embedding/vector stores.
- [ ] Manual: `/self` against a real repo. *(Not run here.)*

## Notes

- M6 evolution consumes the growth drive; decision 008 still blocks M6.
