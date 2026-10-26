# M1-13 — Split the SQLite store

- **Blocked by:** —
- **Blocks:** —
- **Status:** resolved (devin, 2026-10-26)

## Goal

Split `adapters-session-sqlite/src/lib.rs` (453 lines) into store, migration, factory, and helper modules.

## Acceptance

- [x] `lib.rs` is module declarations and re-exports only (9 lines).
- [x] `store.rs` 248, `migrate.rs` 88, `factory.rs` 38, `util.rs` 15; tests 130.
- [x] Public API unchanged; all six tests stay green.
- [x] No behaviour change.

## Notes

- Tests moved to `tests/store.rs`; they use the public store API plus `rusqlite` for the v1 fixture.
- `migrate`, `err`, `lock`, and `new_id` are `pub(crate)` so sibling modules can use them.
