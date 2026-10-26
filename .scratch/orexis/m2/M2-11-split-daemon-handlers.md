# M2-11 — Split the daemon handlers again

- **Blocked by:** —
- **Blocks:** —
- **Status:** resolved (devin, 2026-10-26)

## Goal

M2-09 left `handlers.rs` at 219 lines; M2-05/M2-06 grew it to 386.

## Acceptance

- [x] `handlers/mod.rs` 9, `session.rs` 248, `workspace.rs` 112, `settings.rs` 45.
- [x] Connection loop imports only the re-exports.
- [x] Daemon tests and the gate stay green.

## Notes

- Session handlers: attach, prompt, interrupt, clear overrides, undo, diff.
- Workspace handlers: list, read, artifacts.
- Settings handler: profile/model, permissions, privacy.
