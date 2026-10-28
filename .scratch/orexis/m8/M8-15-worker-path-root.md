# M8-15 — worker-side read resolved under `.orexis/`

- **Status:** open

## Bug

Planner-phase probe (LFM3, team mode): run died with
`read /tmp/.tmpxmWQlB/.orexis/alpha.txt: No such file or directory`.
Either a worker's resolve rooted at `.orexis/` (a cwd/root plumbing
bug) or the model wrote a wrong relative path and the observation
hides which session issued it. Needs the worker session log to tell —
the bench currently can't say which session a failure came from
(separate diagnostic gap).

## Acceptance

- [ ] Repro: identify whether the worker's workspace root or the model's
      path was wrong, with the session log as evidence.
- [ ] Regression test for whichever it turns out to be.
