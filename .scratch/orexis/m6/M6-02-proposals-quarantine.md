# M6-02 — Mutation proposals and quarantine

- **Blocked by:** M6-01
- **Blocks:** M6-04, M6-05
- **Status:** open

## Goal

Data-only mutations proposed at boundaries and staged in quarantine.

## Acceptance

- [ ] Mutation types: prompt/strategy fragment, gene parameter, skill content, recall/reflection policy.
- [ ] Candidate directory with manifest, base hash, proposed diff, and size caps.
- [ ] Growth-drive proposals at session boundaries only; no mid-session.
- [ ] Dedup by content hash; one proposal per session/day.
- [ ] No apply path from this ticket; no executable content.
- [ ] Tests: proposal, dedup, size cap, boundary rule.

## Notes

- MUE-X's six strategies are adapted; code mutation is out of scope.
