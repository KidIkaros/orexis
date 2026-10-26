# M3-08 — M3 gate

- **Blocked by:** M3-01 … M3-07
- **Blocks:** M2
- **Status:** open

## Goal

Close the M3 gate: a real repository task, the adversarial safety suite, and the spec update.

## Acceptance

- [ ] Read, edit, test, and diff a real repository end to end.
- [ ] Path traversal, symlink escape, command injection, and output bombs are rejected.
- [ ] Plan mode cannot write or run shell.
- [ ] `/undo` restores the pre-turn state.
- [ ] Verify-on-stop blocks a false `Done`.
- [ ] `scripts/m3-gate.sh` is green and the seam graph is current.
- [ ] `docs/orexis/specs/M3.md` reflects what shipped.
