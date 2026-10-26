# M3-06 — Verify-on-stop

- **Blocked by:** M3-03, M3-05
- **Blocks:** M3-08
- **Status:** open

## Goal

Refuse a `Done` claim until the configured test/build command passes, unless the user explicitly skips.

## Acceptance

- [ ] A verify command is configured per project.
- [ ] `Done` runs verify and is refused on failure, with the output attached.
- [ ] The model can retry or explicitly skip.
- [ ] A passing verify is recorded in the session.
- [ ] Tests: failing verify blocks, passing verify allows, skip allows.
