# M3-05 — Git tools and turn checkpoints

- **Blocked by:** M3-01
- **Blocks:** M3-06, M3-08
- **Status:** open

## Goal

Read-only git tools plus a shadow-git checkpoint per turn, `/undo`, and diff review before commit.

## Acceptance

- [ ] `git_status` and `git_diff` work read-only.
- [ ] A checkpoint is created before each turn.
- [ ] `/undo` restores files to the checkpoint.
- [ ] A diff is available for review before commit.
- [ ] Checkpoints live outside the worktree (`.orexis/checkpoints`).
- [ ] Tests: edit, undo, diff.
