# M8-10 — Bench breadth tasks + the team-vs-serial gate

- **Blocked by:** M8-09
- **Blocks:** —
- **Status:** open

## Goal

The M8 gate (§9, §10): breadth tasks in the bench (tasks that decompose
into independent subtasks — the shape where parallelism should pay), run
serial and team, compared on pass rate, wall clock, tokens, and cost.
The gate: team achieves ≥15% higher pass rate or ≥25% lower wall-clock at
≤2× tokens — or team mode stays experimental and the cut rule applies.

## Acceptance

- [ ] Breadth tasks exist in the bench corpus with held-out variants.
- [ ] The comparison runs both modes across the model matrix (§10's
      floor: one local small, one local medium, one remote frontier, one
      remote cheap).
- [ ] The result is recorded with the paired bootstrap + noise band.
- [ ] The gate verdict is written into the map (ship or experimental).

## Notes

The 15× token figure is the prior to beat; a team that costs 15× loses
even at a higher pass rate.
