# M5-02 — Six drives

- **Blocked by:** M5-01
- **Blocks:** M5-05, M5-06
- **Status:** open

## Goal

Six slow drives that propose work without touching the prompt.

## Acceptance

- [ ] `Drives` with connection, curiosity, mastery, coherence, rest, growth in `[0,1]`.
- [ ] Baselines, decay, and satisfaction rules; updates append `DriveUpdate`.
- [ ] Proposals at turn/session boundaries only: explore, build, refactor, consolidate/stop, evolution candidate, check-in/summary.
- [ ] No prompt/context mutation from a drive update.
- [ ] Tests: bounds, decay, satisfaction, tension, proposal mapping.

## Notes

- Growth is distinct from mastery and coherence (ADR-0013).
- Connection is attunement, not companionship.
