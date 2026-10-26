# M1-21 — Split the provider catalog

- **Blocked by:** M1-17
- **Blocks:** —
- **Status:** open

## Goal

Split `adapters-provider-catalog/src/lib.rs` (433 lines) into `profile.rs`, `catalog.rs`, and `secrets.rs`, with tests beside their modules.

## Acceptance

- [ ] No file exceeds 250 lines.
- [ ] Public API unchanged; all 5 tests stay green.
- [ ] No behaviour change.
