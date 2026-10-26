# M1-13 — Split the SQLite session store

- **Blocked by:** —
- **Blocks:** —
- **Status:** open

## Goal

Split `crates/adapters-session-sqlite/src/lib.rs` (453 lines) into `store.rs`, `migrate.rs`, and `export.rs`, with tests beside their modules.

## Acceptance

- [ ] No file exceeds 250 lines.
- [ ] The `SessionAppend`/`SessionRead`/`SessionFork` behaviour is unchanged.
- [ ] Migration and export tests move with their modules and stay green.

## Notes

- Purely structural; no schema change.
