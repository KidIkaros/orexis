# M8-10 — Bench breadth tasks + the team-vs-serial gate

- **Blocked by:** M8-09
- **Blocks:** —
- **Status:** claimed
- **Claimed by:** devin, 2026-10-28

## Goal

The M8 gate (§9, §10): breadth tasks in the bench (tasks that decompose
into independent subtasks — the shape where parallelism should pay), run
serial and team, compared on pass rate, wall clock, tokens, and cost.
The gate: team achieves ≥15% higher pass rate or ≥25% lower wall-clock at
≤2× tokens — or team mode stays experimental and the cut rule applies.

## Acceptance

- [x] Breadth tasks exist in the bench corpus with held-out variants.
      (`bench/tasks/breadth.yaml` — three public breadth tasks + two
      held-out variants; `orexis-bench` has the YAML format, fixture
      seeding, and deterministic judges.)
- [ ] The comparison runs both modes across the model matrix (§10's
      floor: one local small, one local medium, one remote frontier, one
      remote cheap). **Manual:** `orexis bench run --tasks
      bench/tasks/breadth.yaml --profile P --model M --modes serial,team
      --out results-<model>.jsonl` per model — needs live providers.
- [x] The result is recorded with the paired bootstrap + noise band.
      (`orexis bench compare --serial a.jsonl --team b.jsonl` — paired
      bootstrap CIs on pass diff and wall gain, token ratio, verdict.)
- [ ] The gate verdict is written into the map (ship or experimental).
      **Manual:** after the matrix run, append the verdict row.

## Landed this session

- Token accounting end to end: `TokenUsage`, `ProviderResponse.usage`,
  `SessionEvent::ModelResponse.usage` (serde-default, old logs read
  fine); openai/anthropic/llamacpp-wire and the fake populate it.
- `orexis-bench` crate + `orexis bench run|compare`. Both modes drive the
  real daemon socket; approvals auto-yes; metrics summed across every
  session the run touched (driver + workers).
- Team preamble: `Daemon::new` tells the model about `board_add` /
  `board_list` when team mode is on — discoverable tools, not hidden ones.

## Notes

The 15× token figure is the prior to beat; a team that costs 15× loses
even at a higher pass rate.
