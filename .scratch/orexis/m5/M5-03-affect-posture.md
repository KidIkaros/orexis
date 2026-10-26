# M5-03 — PAD affect, posture, and glial

- **Blocked by:** M5-01
- **Blocks:** M5-05, M5-06
- **Status:** resolved (devin, 2026-10-26)

## Goal

A fast PAD state that chooses a posture without weakening safety.

## Acceptance

- [x] PAD affect in `[-1,1]`; bounded outcome updates; derived moods.
- [x] Posture fields; `Posture::for_affect` selects them.
- [x] `modulate_drives` is deterministic, stateless, clamped to [0.5, 1.5].
- [x] Posture has no permission surface; affect can only add caution.
- [x] Tests: failure→caution, success→calm, bounded glial, no permission surface.

## Notes

- Moods are derived labels.
- Posture is not a Strategy.
- Posture is persisted as an entity event but not yet consumed by provider routing; that wiring is M5-06.
- `record_affect` appends `AffectUpdate` and, when it changes, `PostureChange`.
