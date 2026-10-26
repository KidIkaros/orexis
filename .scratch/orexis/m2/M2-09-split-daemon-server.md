# M2-09 — Split the daemon connection handler

- **Blocked by:** —
- **Blocks:** —
- **Status:** resolved (devin, 2026-10-26)

## Goal

`crates/daemon/src/server.rs` is 308 lines and the connection loop carries attach, prompt, approve, undo, diff, sessions, and replay. Extract focused handlers.

## Acceptance

- [x] `server.rs` 87, `connection.rs` 139, `handlers.rs` 219; the loop delegates.
- [x] No behaviour change; daemon tests stay green.
- [x] Boundary lint and seam graph stay clean.

## Notes

- Clean-code finding from the M2-02/03 audit.
