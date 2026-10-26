# M6-05 — 100-cycle gate

- **Blocked by:** M6-01 … M6-04
- **Blocks:** —
- **Status:** resolved (devin, 2026-10-26) — automated; manual `/evolve` remains

## Goal

Prove evolution is durable across 100 accelerated cycles.

## Acceptance

- [x] `scripts/m6-gate.sh`: M5 gate + evolution suites + 100-cycle fixture.
- [x] 100 accelerated cycles in `evolution_100_cycles`.
- [x] Ledger verifies after every apply and rollback.
- [x] Every applied entry's seal hash equals the sealed set.
- [x] Executable proposal rejected; quarantine count unchanged.
- [x] Final verification passes.
- [x] Read-only `/evolve` TUI surface reads the ledger. *(Real-repo inspection remains manual.)*

## Notes

- Evolution calibration after 100 cycles remains fog.
