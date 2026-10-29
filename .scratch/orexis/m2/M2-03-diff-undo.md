# M2-03 — Diff viewer and undo

- **Blocked by:** M2-01
- **Blocks:** M2-06
- **Status:** resolved (2026-10-29) — hunk-aligned split diff, automatic full-screen (>40 lines), and undo wired

## Goal

Review changes side-by-side and restore a checkpoint.

## Acceptance

- [x] Diff viewer with unified/split toggle and line coloring (add/del/hunk/meta). *(Hunk-aligned row-by-row split with `.pad` cells for unbalanced blocks.)*
- [x] Diffs over the viewport open full-screen automatically (`AppState::diff_expanded` when `> 40` lines, plus manual expand/close toggle).
- [x] Undo restores the latest checkpoint and confirms with a toast (daemon `restored checkpoint` info, CSS fade).
- [x] Diff and undo work from the keyboard (`d`, `Ctrl/Cmd+Z`) and buttons.

## Notes

- `GitCheckpointStore::diff` now returns a unified `git diff` (was `--stat`), capped at 64 KB.
- The daemon answers `ClientMsg::Diff` with `ServerMsg::Diff` from the latest checkpoint.
- The split view filters `-`/`+` lines into two panes; hunk alignment is approximate.
- Undo is also available as a status-bar button; the toast comes from the daemon's info event.
