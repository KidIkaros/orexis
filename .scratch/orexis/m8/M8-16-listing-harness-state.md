# M8-16 — the model's listing shows harness state

- **Status:** fixed; gate re-run pending
- **Found by:** M8-15

## Bug

`list_dir` returns every entry in the directory, harness state included. In
the bench that is the whole daemon: `bench.db`, `board.db`, `bus.db`,
`entity.db`, `bench.sock`, plus the harness's own `.orexis/`. The
`Workspace` port's `list()` hides `.git`, `.orexis`, `target`, and
`node_modules`; the model-facing `list_dir` hides nothing, so the two
disagree about what the workspace contains.

M8-15 is the observed cost: a model read the listing, saw `.orexis/` above
`alpha.txt`, and asked for `.orexis/alpha.txt`.

## Acceptance

- [ ] Red test: a `list_dir` at the workspace root over a fixture with
      `.orexis/` and a daemon state file must not advertise them.
- [ ] Decide the rule and record it here: which names the model-facing
      listing hides (at least `.orexis`; `target/` and `node_modules/` cost
      steps in every repo, but hiding them hides legitimate browsing).
- [ ] The bench keeps its state out of the fixture workspace, so the gate
      measures the fixture and not the daemon.

## Why it matters now

The M8 gate's pass rate is measured through this listing. Noise in it is a
step tax on every worker, and the small models the gate runs hit it hardest.

## Resolution

Rule: the model-facing `list_dir` hides `.orexis` and `.git` only; `target/`
and `node_modules/` stay listable (browsing them is legitimate).
Test: `list_dir_hides_harness_state_but_not_build_dirs`. The bench now puts
daemon state (`bench.db`, `board.db`, `bus.db`, `entity.db`, `bench.sock`) in
a state dir beside a `workspace/` subdirectory holding only the fixture.
