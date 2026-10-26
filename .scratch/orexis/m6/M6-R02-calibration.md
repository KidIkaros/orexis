# M6-R02 — Evolution calibration after 100 cycles

- **Type:** research
- **Blocked by:** M6-05
- **Blocks:** — (not an M6 gate item)
- **Status:** open
- **Related:** ADR-0015, M6-05

## Goal

Calibrate evolution thresholds and cadence from real 100-cycle data. The
gate proves durability, not that the numbers are right.

## Metrics

- proposal acceptance rate and rejection reasons;
- rollback rate by trigger;
- behavioral delta distribution vs the noise band;
- cost/token/time per cycle;
- sealed-set attempts and quarantine false positives;
- operator review time for `quarantine`/`core`.

## Acceptance

- [ ] 100-cycle run on a real repository with recorded metrics.
- [ ] Noise band and soak duration justified from the data.
- [ ] Cadence (one/session/day) reviewed.
- [ ] Tier defaults reviewed (especially `skills` auto-apply).
- [ ] Recommendation: keep, adjust, or tighten; new ADR if defaults change.

## Notes

- Do not change defaults from a single run.
- Calibration is separate from the durability gate.
