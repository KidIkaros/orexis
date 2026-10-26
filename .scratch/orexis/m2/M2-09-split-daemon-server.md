# M2-09 — Split the daemon connection handler

- **Blocked by:** —
- **Blocks:** —
- **Status:** open

## Goal

`crates/daemon/src/server.rs` is 308 lines and the connection loop carries attach, prompt, approve, undo, diff, sessions, and replay. Extract focused handlers.

## Acceptance

- [ ] `server.rs` under 250 lines; connection loop delegates to handlers.
- [ ] No behaviour change; daemon tests stay green.
- [ ] Boundary lint and seam graph stay clean.

## Notes

- Clean-code finding from the M2-02/03 audit.
