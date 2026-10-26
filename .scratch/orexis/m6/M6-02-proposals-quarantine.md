# M6-02 — Mutation proposals and quarantine

- **Blocked by:** M6-01
- **Blocks:** M6-04, M6-05
- **Status:** resolved (devin, 2026-10-26)

## Goal

Data-only mutations proposed at boundaries and staged in quarantine.

## Acceptance

- [x] Mutation types: prompt fragment, gene parameter, skill content, recall policy.
- [x] Candidate directory with manifest, base hash, proposed content, and 64 KB cap. *(A rendered diff needs the base artifact; that is M6-03.)*
- [x] Growth-drive proposals require `boundary: true` and growth >= 0.6.
- [x] Dedup by content hash; one proposal per session and per day.
- [x] No apply path; executable-marker guard rejects scripts.
- [x] Tests: proposal/list/get, dedup, cadence, boundary, executable guard, size cap.

## Notes

- MUE-X's six strategies are adapted; code mutation is out of scope.
- The executable-marker guard is heuristic; full static safety is M6-03.
- Quarantine is local staging, not the corridor ledger; no apply path exists here.
