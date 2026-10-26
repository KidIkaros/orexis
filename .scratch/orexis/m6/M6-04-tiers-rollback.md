# M6-04 — Tiers, apply, and rollback

- **Blocked by:** M6-01 … M6-03
- **Blocks:** M6-05
- **Status:** resolved (devin, 2026-10-26)

## Goal

Enforce tier defaults and apply/rollback with a signed ledger.

## Acceptance

- [x] Tier defaults and rules: off/skills/quarantine/core.
- [x] Apply requires a passed validation report; corridor commits and signs.
- [x] `auto_rollback` triggers for safety/chain/signature/unrecoverable/verify.
- [x] Regression rolls back automatically for skills; core requires human.
- [x] Revert commit + ledger `Revert`; quarantine candidate remains.
- [x] Tests: off, skills, core signature, invalid validation, rollback; end-to-end fixture.

## Notes

- The sealed set is immutable at every tier.
- `signed` is a caller-supplied boolean; candidate signature verification is not built.
- Human approval is a boolean; no UI/daemon surface yet.
- Rollback triggers are policy functions; no monitor wiring yet.
