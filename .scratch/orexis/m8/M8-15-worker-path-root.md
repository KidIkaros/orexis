# M8-15 — worker-side read resolved under `.orexis/`

- **Status:** closed (2026-10-28) — the model's path was wrong; the root is
  the project root.

## Bug

Planner-phase probe (LFM3, team mode): run died with
`read /tmp/.tmpxmWQlB/.orexis/alpha.txt: No such file or directory`.
Either a worker's resolve rooted at `.orexis/` (a cwd/root plumbing
bug) or the model wrote a wrong relative path and the observation
hides which session issued it.

## Evidence

Reproduced on Qwen3.5-2B (llamacpp, `--modes team`, `probe-decompose`).
The session log (`bench.db` → `event`) names both the session and the
argument as sent:

```
01M628VMGQ… tool_call   {"name":"list_dir","args":"{\"path\":\".\"}"}
01M628VMGQ… observation <tool_result name="list_dir">
                        .orexis/
                        alpha.txt  48
                        bench.db  4096
                        …
01M628VMGQ… tool_call   {"name":"read_file","args":"{\"path\":\".orexis/alpha.txt\"}"}
01M628VMGQ… error       read <root>/.orexis/alpha.txt: No such file or directory
```

The failing call is in the **driver** session — M8-15's guess (a worker
re-rooted under `.orexis/`) is not what happened. `resolve()` joined the
argument to the project root exactly as written
(`crates/adapters-tools/src/lib.rs`), and one `LocalTools` is built in
composition at `project_root` and shared by every session and worker, so
nothing can re-root a worker's tools. The model built the path from a
listing that showed the harness's own `.orexis/` beside the task files.

Two probes reproduced it; the second also showed the driver's error
ending its session while the plan worker kept running, so the run
drained only at the bench's timeout. That is the supervisor's territory,
not this ticket's.

## Regression

`crates/daemon/tests/worker.rs`:

- `a_workers_relative_path_resolves_under_the_project_root` — with
  `.orexis/` present in the workspace, a worker's `read_file
  {"path":"alpha.txt"}` resolves at the root and returns the fixture.
- `a_path_naming_the_harness_dir_is_taken_literally` — pins the message
  shape the diagnosis leaned on: the error names the root joined with the
  argument as sent.

The bench side of the diagnostic gap is closed too: rows now carry
`sessions` and `errors` (`<session_id>: <message>`), so a stop reason
alone never has to answer "who failed" again.

## Finding filed

M8-16 — the listing a model sees includes the harness's `.orexis/`, and in
the bench it also includes the daemon's state files.
