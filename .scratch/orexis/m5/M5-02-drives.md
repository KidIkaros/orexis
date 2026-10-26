# M5-02 — Six drives

- **Blocked by:** M5-01
- **Blocks:** M5-05, M5-06
- **Status:** resolved (devin, 2026-10-26)

## Goal

Six slow drives that propose work without touching the prompt.

## Acceptance

- [x] `Drives` with six drives in `[0,1]`; clamp and baseline 0.5.
- [x] Baseline decay and satisfaction; `record_outcome`/`decay` append `DriveUpdate`.
- [x] Pure proposals (explore, build, refactor, consolidate, evolve, check-in) for boundary callers.
- [x] Pure drive logic; no prompt/context access.
- [x] Tests: bounds, decay, satisfaction, tension, proposal mapping, appended updates.

## Notes

- Growth is distinct from mastery and coherence (ADR-0013).
- Connection is attunement, not companionship.
- Baseline is the default 0.5 for now; per-entity baselines are a later refinement.
- Turn-loop wiring is M5-06; the use case is pure and tested with a recording append.
