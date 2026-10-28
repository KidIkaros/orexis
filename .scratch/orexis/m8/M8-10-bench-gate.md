# M8-10 — Bench breadth tasks + the team-vs-serial gate

- **Blocked by:** M8-09
- **Blocks:** —
- **Status:** resolved (2026-10-28) — the gate ran; team mode stays
  experimental.
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
      **One of four run:** Qwen3.5-2B (llamacpp, local small), five
      public + held-out pairs, 200 s cap.
- [x] The result is recorded with the paired bootstrap + noise band.
      (`orexis bench compare --serial a.jsonl --team b.jsonl` — paired
      bootstrap CIs on pass diff and wall gain, token ratio, verdict.)
- [x] The gate verdict is written into the map (ship or experimental).
      **Experimental** — see below.

## The gate run (2026-10-28)

`bench-results-qwen35-gate.jsonl`, Qwen3.5-2B-Q4_K_M (llamacpp), the five
breadth tasks (three public, two held-out), `--timeout-secs 200`:

| Task | Serial | Team | Serial wall | Team wall | Serial tok | Team tok |
|---|---|---|---|---|---|---|
| fix-four-modules | pass | pass | 10.3 s | 200.0 s (cap) | 7,736 | 18,317 |
| write-four-reports | pass | pass | 8.6 s | 200.0 s (cap) | 5,199 | 11,829 |
| rename-three-constants | pass | pass | 11.5 s | 200.1 s (cap) | 16,032 | 11,803 |
| fix-four-modules-ho | **fail** | pass | 4.8 s | 16.4 s | 3,528 | 11,534 |
| write-four-reports-ho | pass | pass | 7.3 s | 200.1 s (cap) | 5,093 | 14,179 |

`bench compare`: pass 4/5 → 5/5, mean diff +0.200, CI [+0.000, +0.600];
wall mean gain −1,720%, CI [−2,319%, −922%]; tokens 1.80×; **verdict
experimental**.

Three readings matter:

- **The team arm is not a team yet.** In the runs inspected, the driver
  did the work itself — `list_dir`, four `read_file`, four `edit_file`,
  four verification reads, `Done`, in ~15 s — while the plan worker
  claimed the seeded task, called `board_add`, and stalled until the
  hub's 300 s approval clock expired (`approval timed out`, M8-17). The
  board never drained, so four of five runs ended at the cap with the
  work already done by the driver.
- The one flipped pair (serial fails `fix-four-modules-ho`) is the M8-16
  listing trap: the serial model read `.orexis/alpha.txt` and ended its
  session. Team "won" that pair for the wrong reason.
- The token ratio (1.80×) is inside the budget, but it is the driver's
  serial work plus a stalled worker, not parallel work.

So the verdict is experimental on a stronger ground than the raw numbers:
until M8-17 is fixed the gate cannot measure team mode at all. The wall
column is a 300 s approval timeout, and the pass column is the driver's
serial result.

## What the next gate run needs

1. M8-17 (worker approvals) — without it no worker can write to the board,
   so no team run completes.
2. M8-16 (the listing noise) — it moves both arms' pass rates.
3. The other three matrix classes, so the verdict is not one model's.

An earlier version of the verdict read the point estimate and shipped a
20× slower team on that single flip; the gate now reads each axis' noise
band (`a_lucky_flip_does_not_ship_a_slower_team`). Small-N means are not
a win.

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
