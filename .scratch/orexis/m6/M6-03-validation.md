# M6-03 — Validation ladder and evaluation

- **Blocked by:** M6-01
- **Blocks:** M6-04, M6-05
- **Status:** open

## Goal

A deterministic ladder that rejects unsafe or regressive candidates.

## Acceptance

- [ ] Schema, seal, and static-safety checks.
- [ ] Deterministic mutation-specific tests.
- [ ] Behavioral evaluation: paired baseline/candidate with a noise band.
- [ ] 24h quarantine soak; human review for `quarantine`/`core`.
- [ ] `ValidationReport` records each stage and the rejection reason.
- [ ] Tests: sealed change, capability addition, regression, pass.

## Notes

- No apply in this ticket.
