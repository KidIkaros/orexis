# M6-04 — Tiers, apply, and rollback

- **Blocked by:** M6-01 … M6-03
- **Blocks:** M6-05
- **Status:** open

## Goal

Enforce tier defaults and apply/rollback with a signed ledger.

## Acceptance

- [ ] Fresh install `off`; `skills` opt-in; `quarantine` stage-only; `core` signed + human.
- [ ] Apply only after the full ladder; git commit + signed ledger entry.
- [ ] Automatic rollback on safety/chain/signature/unrecoverable failure.
- [ ] Automatic `skills` rollback on regression; human `core` rollback.
- [ ] Revert commit + ledger `Revert`; candidate returns to quarantine.
- [ ] Tests: off refuses, skills applies, core needs approval, rollback restores.

## Notes

- The sealed set is immutable at every tier.
