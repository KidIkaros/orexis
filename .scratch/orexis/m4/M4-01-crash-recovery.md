# M4-01 — Crash recovery

- **Blocked by:** —
- **Blocks:** M4-02, M4-04
- **Status:** resolved (devin, 2026-10-26)

## Goal

A SIGKILL mid-turn must leave a session that can be replayed and safely
recovered; the daemon must not pretend the turn finished.

## Acceptance

- [x] `SessionEvent::Recovered` is durable and kind `recovered`.
- [x] Attach marks an unfinished turn once; an active turn is not marked.
- [x] A second attach does not append another recovery event.
- [x] TUI/desktop show recovery as info and clear live state.
- [x] Completed-turn restart test still passes.
- [x] No auto-resume: the client decides whether to prompt again.

## Notes

- Unfinished = last non-info/compaction event is not Done/Error/Interrupted/Recovered.
- Recovery is skipped while the runtime has an active turn handle.
- Checkpoints and undo are unaffected; the recovered turn can be undone.
