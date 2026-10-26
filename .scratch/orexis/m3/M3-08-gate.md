# M3-08 — M3 gate

- **Blocked by:** M3-01 … M3-07
- **Blocks:** M2
- **Status:** resolved (devin, 2026-10-26)

## Goal

Close the M3 gate: a real repository task, the adversarial safety suite, and the spec update.

## Acceptance

- [x] Read, edit, test, and diff a real repository end to end (`real_task.rs`).
- [x] Path traversal, symlink escape, command injection, and output bombs are rejected (`adversarial.rs`).
- [x] Plan mode cannot write or run shell.
- [x] `/undo` restores the pre-turn state.
- [x] Verify-on-stop blocks a false `Done`.
- [x] `scripts/m3-gate.sh` is green and the seam graph is current.
- [x] `docs/orexis/specs/M3.md` reflects what shipped.

## Notes

- The real task is driven through the daemon with a scripted provider: read → edit → `cargo test` → `git_diff` → Done → verify.
- The adversarial suite covers plan mode, traversal, symlink, output bombs, hardline shell, and undo.
- Live-provider runs and a hand-driven TUI session remain manual; the gate script prints the commands.
