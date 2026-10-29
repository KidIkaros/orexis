# M8-17 — a worker's board write waits on an approval no client can answer

- **Status:** routing fixed (rule a); gate re-run pending — found by the M8 gate run (M8-10)
- **Blocks:** any honest M8 team measurement

## Bug

In team mode a worker's tool call that needs approval never gets one:
approval requests are routed to the connection attached to *that* session
(`crates/daemon/src/connection.rs`: `rt.id != session_id` → dropped), and
nothing attaches to a worker's session. The worker waits out the hub's
300 s clock and the turn fails.

Observed on a one-task team run, `OREXIS_BENCH_KEEP=1`,
`--timeout-secs 340`:

```json
"stop_reason": "read <root>/.orexis/alpha.txt: No such file or directory",
"errors": [
  "01M62EN1EJ…: read <root>/.orexis/alpha.txt: No such file or directory",
  "01M62EN4KMDJ696J8Q8954W05T: approval timed out"
],
"wall_ms": 312770
```

The plan worker's transcript stops at `tool_call board_add` — no
observation, no `Done` — and the board never gains the subtask
(`board.db` holds only the seeded plan task, `status=running`). The
driver, meanwhile, does the whole task itself: `list_dir`, four
`read_file`, four `edit_file`, four verification reads, `Done`, in ~15 s.
The board then never drains, so the run idles to the bench cap and the
"team" arm is really the driver's serial work plus a 300 s wait.

The earlier board (`.tmplCvveS`, a 300 s-cap run) shows the same failure
from the other side: `Fix alpha.txt: status -> "active" — failed:
approval timed out`.

## Why it was invisible until now

The driver's approvals do arrive (the bench client is attached to the
driver), so every run looked like "the model wouldn't decompose" —
M8-14's reading. The plan phase (M8-14) does get a worker to claim the
task; the worker's first board write is where it dies. Four of five team
runs in the gate hit this; the fifth's driver also finished alone.

## Acceptance

- [ ] Red test: a worker turn whose tool needs approval reaches its
      observation without a client attached to the worker's session.
- [ ] Decide the routing rule and record it here (ADR if it changes
      invariant 6, "permissions are a ruleset at the tool layer"):
      a. worker approvals surface on the spawning session's connection,
         labelled `worker-…: board_add`;
      b. board tools are policy-allowed for workers (a board mutation is
         not a workspace mutation; the strategy filter already narrows);
      c. both — read-only tools free, mutating tools ask on the parent.
- [ ] The bench answers worker approvals only if the chosen rule says a
      client must.
- [ ] Re-run the M8 gate (M8-10) after the fix; the verdict may change.

## Resolution (rule a)

The team loop is daemon-wide, so there is no single spawning session to
route to. `spawn_worker_turn` marks the worker's session on the
`ApprovalHub`; `ApprovalHub::surfaces` shows a worker's request on every
connection, and the request's `session_id` names the worker. Invariant 6
is untouched: the ruleset still decides what asks; only the audience
changed. Unit test: `a_workers_request_surfaces_on_any_connection`.
Open: the bench client must answer worker approvals (next step), then the
M8-10 gate re-run.

## Repro

```bash
sed -n '5,37p' bench/tasks/breadth.yaml > /tmp/one-task.yaml
OREXIS_BENCH_KEEP=1 ./target/debug/orexis bench run \
  --tasks /tmp/one-task.yaml --profile llamacpp \
  --model Qwen3.5-2B-Q4_K_M.gguf --modes team \
  --timeout-secs 340 --out /tmp/keep.jsonl
sqlite3 <kept>/board.db 'select * from task'
sqlite3 <kept>/bench.db "select session_id,kind from event where kind='error'"
```