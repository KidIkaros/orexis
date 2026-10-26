# M5-03 — PAD affect, posture, and glial

- **Blocked by:** M5-01
- **Blocks:** M5-05, M5-06
- **Status:** open

## Goal

A fast PAD state that chooses a posture without weakening safety.

## Acceptance

- [ ] `Affect { valence, arousal, dominance }` in `[-1,1]`; bounded update rules from outcomes.
- [ ] `Posture` fields: model tier, thinking, repair budget, verification, risk appetite, exploration.
- [ ] Glial modulation is deterministic and stateless; multipliers clamped.
- [ ] Affect may add caution, never turn `deny` into `allow` or bypass plan mode.
- [ ] Tests: failure→caution, success→calm, safety floor, no prompt mutation.

## Notes

- Moods are derived labels.
- Posture is not a Strategy.
