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
