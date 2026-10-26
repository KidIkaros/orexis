# M5-05 — Identity distillation and /self

- **Blocked by:** M5-01, M5-04
- **Blocks:** M5-06
- **Status:** resolved (devin, 2026-10-26)

## Goal

Evidence-backed traits distilled from an append-only diary, visible in `/self`.

## Acceptance

- [x] Diary append-only; entries may reference a session seq.
- [x] Traits require evidence; confidence decays. *(Last-seen is not stored yet.)*
- [x] `render_identity` generates `IDENTITY.md` text; file writing is a later surface.
- [x] `/self` shows drives, PAD/mood, posture, traits, memory count, recent diary.
- [x] Identity injected on the first turn only; test proves the second turn's system prompt omits it.
- [x] Tests: evidence requirement, decay, rendering, `/self`, first-turn-only injection.

## Notes

- `SOUL.md` is not a thing; identity is emergent.
- Desktop entity view can follow in a later ticket.
- `IDENTITY.md` is rendered, not yet written to disk.
- Trait last-seen and model-switch persistence are M5-06 gate items.
