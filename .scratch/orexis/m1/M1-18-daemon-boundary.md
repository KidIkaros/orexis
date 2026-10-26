# M1-18 — Daemon composition boundary

- **Blocked by:** —
- **Blocks:** M2, M3
- **Status:** resolved (devin, 2026-10-26)

## Goal

Stop the daemon from constructing the SQLite adapter. Inject a `SessionFactory` from composition, list sessions from the database, and record an `Interrupted` event.

## Acceptance

- [x] `SessionRegistry` takes an `Arc<dyn SessionFactory>`; it constructs no adapter.
- [x] Composition provides `SqliteSessionFactory` (open, open_session, list).
- [x] `ListSessions` includes sessions that exist in the database but are not attached (restart test).
- [x] `Interrupt` appends `SessionEvent::Interrupted` and broadcasts it before aborting the turn.
- [x] `derive_messages` ignores `Interrupted`; it is redacted and versioned.
- [x] Daemon tests use the injected factory and still pass.

## Notes

- `SessionFactory`/`SessionRefs` live in `orexis-ports`; `SqliteSessionFactory` implements them.
- `SessionRuntime` holds `append`/`read` trait objects, not a concrete store.
