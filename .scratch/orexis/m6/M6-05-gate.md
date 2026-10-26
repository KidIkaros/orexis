# M6-05 — 100-cycle gate

- **Blocked by:** M6-01 … M6-04
- **Blocks:** —
- **Status:** open

## Goal

Prove evolution is durable across 100 accelerated cycles.

## Acceptance

- [ ] `scripts/m6-gate.sh`: M5 gate + evolution suites.
- [ ] 100 accelerated proposal/validate/apply/rollback cycles.
- [ ] Zero unrecoverable states; every failure returns to the last signed good state.
- [ ] Sealed set unchanged in every applied candidate.
- [ ] Quarantine never auto-applies executable content.
- [ ] Ledger chain/signatures verify after 100 cycles.
- [ ] Manual: inspect `/evolve` and the ledger on a real repo.

## Notes

- Evolution calibration after 100 cycles remains fog.
