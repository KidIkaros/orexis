# M4-04 — M4 gate

- **Blocked by:** M4-01 … M4-03
- **Blocks:** —
- **Status:** resolved (devin, 2026-10-26) — automated; manual real-repo run remains

## Goal

Prove recovery, audit, and quarantine together.

## Acceptance

- [x] `scripts/m4-gate.sh`: M3 gate + SIGKILL, audit, and skills suites.
- [x] SIGKILL mid-turn fixture (`kill_mid_turn_recovers`).
- [x] Audit fixtures: approve + always, deny.
- [x] Tampered-skill quarantine fixture (skills tests).
- [ ] Manual: real repo kill/restart run. *(Not run here.)*
