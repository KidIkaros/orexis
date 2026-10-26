# M6-03 — Validation ladder and evaluation

- **Blocked by:** M6-01
- **Blocks:** M6-04, M6-05
- **Status:** resolved (devin, 2026-10-26)

## Goal

A deterministic ladder that rejects unsafe or regressive candidates.

## Acceptance

- [x] Schema, seal, and static-safety stages (sealed targets rejected).
- [x] Deterministic stage requires non-empty tests and all passing.
- [x] Behavioral stage uses paired baseline/candidate and a configurable noise band.
- [x] Soak and human stages are policy-driven (24h, human required).
- [x] `ValidationReport` records every stage, detail, and overall pass/fail.
- [x] Tests: full pass, sealed target/seal change, regression, human pending, executable, missing tests.

## Notes

- No apply in this ticket.
- The behavioral result is supplied by the caller; the eval runner is M6-05.
- Soak/human are inputs to the ladder; M6-04 enforces them before apply.
