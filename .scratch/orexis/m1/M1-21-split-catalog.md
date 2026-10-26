# M1-21 — Split the provider catalog

- **Blocked by:** M1-17
- **Blocks:** —
- **Status:** resolved (devin, 2026-10-26)

## Goal

Split `adapters-provider-catalog/src/lib.rs` (433 lines) into profile, catalog, and secret modules.

## Acceptance

- [x] No file exceeds 250 lines: `profile.rs` 227, `catalog.rs` 102, `secrets.rs` 39, `lib.rs` 9.
- [x] Public API unchanged; all five tests stay green.
- [x] No behaviour change.

## Notes

- Tests moved to `tests/catalog.rs` (73 lines).
- `profile.rs` owns the profile schema; `catalog.rs` owns precedence and model resolution; `secrets.rs` owns `SecretStore`/`EnvSecrets`/`MemorySecrets`.
