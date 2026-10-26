# M2-03 — Diff viewer and undo

- **Blocked by:** M2-01
- **Blocks:** M2-06
- **Status:** in progress (devin, 2026-10-26) — diff/undo wired; automatic full-screen and exact split alignment remain; Tauri shell unbuilt

## Goal

Review changes side-by-side and restore a checkpoint.

## Acceptance

- [x] Diff viewer with unified/split toggle and line coloring (add/del/hunk/meta). *(Split is a two-pane filter, not hunk-aligned.)*
- [ ] Diffs over the viewport open full-screen automatically. *(An explicit expand/close button works.)*
- [x] Undo restores the latest checkpoint and confirms with a toast (daemon `restored checkpoint` info, CSS fade).
- [x] Diff and undo work from the keyboard (`d`, `Ctrl/Cmd+Z`) and buttons.

## Notes

- `GitCheckpointStore::diff` now returns a unified `git diff` (was `--stat`), capped at 64 KB.
- The daemon answers `ClientMsg::Diff` with `ServerMsg::Diff` from the latest checkpoint.
- The split view filters `-`/`+` lines into two panes; hunk alignment is approximate.
- Undo is also available as a status-bar button; the toast comes from the daemon's info event.
