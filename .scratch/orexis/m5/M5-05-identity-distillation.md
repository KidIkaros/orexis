# M5-05 — Identity distillation and /self

- **Blocked by:** M5-01, M5-04
- **Blocks:** M5-06
- **Status:** open

## Goal

Evidence-backed traits distilled from an append-only diary, visible in `/self`.

## Acceptance

- [ ] Diary append-only; entries reference entity/session seq.
- [ ] Traits: statement, evidence refs, confidence, last-seen, decay; no evidence → no trait.
- [ ] `IDENTITY.md` generated from traits; no agent-authored source of truth.
- [ ] `/self` shows drives, PAD, moods, traits, recent diary.
- [ ] Identity injected at session start only; mid-session changes do not alter the prompt.
- [ ] Tests: evidence requirement, decay, model switch, no mid-session leak.

## Notes

- `SOUL.md` is not a thing; identity is emergent.
- Desktop entity view can follow in a later ticket.
