# M3-03 — Sandboxed shell

- **Blocked by:** M3-02
- **Blocks:** M3-06, M3-08
- **Status:** resolved (devin, 2026-10-26)

## Goal

A `shell` tool confined to the project root: timeout, per-stream output caps, process group, no orphans, no inherited secrets.

## Acceptance

- [x] cwd is the project root (or a resolved subdirectory); commands run in a new process group.
- [x] Timeout (120 s default, 1–600 s) kills the group with `kill(-pid, SIGKILL)`.
- [x] stdout/stderr cap at 64 KB each with a marker; the pipe is drained after the cap.
- [x] A hardline floor blocks `sudo`, root deletion, `mkfs`, raw device writes, fork bombs, shutdown, reboot; the policy gate adds `ask`.
- [x] The environment is cleared and rebuilt from an allowlist; provider keys never enter.
- [x] Seven shell tests: run, exit code, timeout, output bomb, hardline, env scrub, cwd escape.

## Notes

- `run_shell` runs in `spawn_blocking`; the daemon is a tokio runtime.
- The orphan check is code-level (process group + kill); a `ps`-based assertion is a later hardening.
- `shell` defaults to `Ask` in the policy; the daemon approval hub gates it.
