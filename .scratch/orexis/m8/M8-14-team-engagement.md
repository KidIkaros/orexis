# M8-14 — Make team mode actually engage (decision)

- **Status:** open — evidence gathering

## Question

Both bench models (LFM3-1.7B, MiniCPM5-1B) never call board_add
spontaneously (workers=0 all 20 runs) — but comply instantly when told
to. Team mode measured "models won't decompose", not "parallel loses".

## Options

1. Hardened preamble contract (must-decompose rule).
2. Planner phase: a plan worker produces the board first (M8 shape).
3. Seeded plan in bench only (less honest).

## Evidence so far

Probe `bench/tasks/probe-board.yaml` (explicit instruction): board_add
fired, worker spawned, done in 18s — wiring proven, willingness absent.

## Result — hardened preamble experiment (option 1)

Preamble changed to a must-decompose RULE in Daemon::new. Probe
`probe-decompose.yaml` (the 4-file breadth task, team mode, 300s cap):

- MiniCPM5-1B: workers=0, 13 tool calls (did the work serially), timeout.
- LFM3-1.7B: workers=0, 13 tool calls, timeout (git noise in stop_reason,
  also confirms M8-13).

Explicit instruction still works on both (workers spawn immediately).
Conclusion: conditional prompt contracts do not reliably trigger
decomposition at the ≤2B scale — small models default to doing the work
themselves. Option 2 (planner phase / structural decomposition) is the
remaining honest path; prompt hardening alone does not engage team mode.

## Third model — Qwen3.5-2B (bench-results-qwen35.jsonl)

Same outcome: serial 4/5 pass; team 2/5; workers=0 all runs. Three
architectures at 1-2B (MiniCPM5, LFM3, Qwen3.5) — zero spontaneous
board_add. Decomposition must be structural.

## Decision direction — planner phase (option 2)

Team + prompt → daemon seeds board task { title: <task>, strategy:
"plan" }. The plan worker's entire job is board_add per subtask —
unconditional instruction, the shape proven to work (probe-board).
Driver session supervises and synthesizes. Remote/larger models
(refuseless glm-5.3, openrouter nemotron-ultra) will re-test whether
scale changes spontaneous engagement — the planner phase is correct
regardless: it removes a behavioral dependency on the model choosing
to decompose.

## Implemented — planner phase (option 2)

`handlers::session::prompt` seeds `board.create(title=<prompt first
line>, strategy="plan")` before spawning the driver turn when team is
on. Driver preamble rewritten: oversight role (board_list/board_add/
synthesize). Test: `attach::team_prompt_seeds_a_plan_task`.

Live checks: LFM3-1.7B probe — the seeded plan task was claimed and the
plan worker spawned (workers=1); run still timed out on a worker-side
path error (`.orexis/alpha.txt` read — separate bug, ticket-worthy).
Refuseless glm-5.3-abliterated followed the *contract* on its own
(wrk=4 via board_add, no seeding — the binary predated this change):
scale does change spontaneous engagement. Planner phase is correct for
both cases.

## Planner phase under the gate (2026-10-28)

The plan task is claimed on every team run — engagement is now structural,
as intended. What the plan worker then does is stall: its first `board_add`
waits on an approval no client can answer, and the hub's 300 s clock ends
the turn (`approval timed out`, M8-17). The board never gains the subtasks,
never drains, and the driver does the work itself. So the gate's team arm
measures the driver, not the team, until M8-17 is fixed. The decision
itself (option 2) stands; it is the approval routing beneath it that
blocks the evidence.
