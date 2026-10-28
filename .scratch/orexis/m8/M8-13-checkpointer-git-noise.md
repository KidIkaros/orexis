# M8-13 — checkpointer/diff leaks "Not a git repository" into runs

- **Status:** done — devin
- **Landed:** `git_status`/`git_diff` short-circuit to a plain
  `"not a git repository"` observation when `root/.git` is absent;
  git's stderr no longer propagates as a tool error. Regression tests:
  `git.rs::non_repo_degrades_to_a_clean_message`, plus the updated
  `tools.rs` integration test. The checkpoint shadow-git was never the
  leak — it owns `.orexis/checkpoints.git`.

## Bug

Bench run `fix-four-modules-ho serial` recorded stop_reason beginning
`git diff: warning: Not a git repository` — the checkpoint/diff path
invokes git on a fixture workspace that isn't a repo. stderr noise
escapes into run metadata; the tool should degrade quietly.

## Acceptance

- [x] Red repro, then regression test.
- [x] No git noise on non-repo workspaces.
