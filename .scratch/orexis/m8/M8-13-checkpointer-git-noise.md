# M8-13 — checkpointer/diff leaks "Not a git repository" into runs

- **Status:** open

## Bug

Bench run `fix-four-modules-ho serial` recorded stop_reason beginning
`git diff: warning: Not a git repository` — the checkpoint/diff path
invokes git on a fixture workspace that isn't a repo. stderr noise
escapes into run metadata; the tool should degrade quietly.

## Acceptance

- [ ] Red repro, then regression test.
- [ ] No git noise on non-repo workspaces.
