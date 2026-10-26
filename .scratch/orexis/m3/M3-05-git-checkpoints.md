# M3-05 — Git tools and turn checkpoints

- **Blocked by:** M3-01
- **Blocks:** M3-06, M3-08
- **Status:** resolved (devin, 2026-10-26)

## Goal

Read-only git tools plus a shadow-git checkpoint per turn, `/undo`, and diff review before commit.

## Acceptance

- [x] `git_status` and `git_diff` work read-only (outside a repo they fail clearly).
- [x] A checkpoint is created before each turn (`SessionEvent::Checkpoint`).
- [x] `/undo` restores files to the latest checkpoint (`ClientMsg::Undo`).
- [x] A diff is available for review (`git_diff`, and `Checkpointer::diff`).
- [x] Checkpoints live outside the worktree (`.orexis/checkpoints.git`).
- [x] Tests: checkpoint round-trip, untracked removal, diff, daemon undo.

## Notes

- `GitCheckpointStore` uses a shadow git dir with an `info/exclude` for `.orexis`, `target`, `node_modules`.
- `restore` resets tracked files and deletes files not present in the checkpoint tree; `.git` and `.orexis` are skipped.
- The daemon's `Undo` finds the latest `Checkpoint` event and restores it; a `checkpoint diff` command is a later addition.
