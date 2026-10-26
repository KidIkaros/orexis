# M1-01 — SQLite session store (ADR-0001)

- **Blocked by:** —
- **Blocks:** M1-05, M1-06, M1-07, M1-08
- **Status:** resolved (devin, 2026-10-25)

## Goal

Implement the ADR-0001 store in `orexis-adapters-session-sqlite`: `session`, `event`, and `generation` tables; `SessionAppend`/`SessionRead`/`SessionFork`; lazy read-time migration; `orexis migrate`; request hashes with a real digest.

## Acceptance

- [x] Append and read round-trip preserves `seq` order and `event_id`.
- [x] Fork creates a new session with `parent_id` + `fork_seq` and copies the prefix.
- [x] `derive_messages` + `verify_replay` pass over a recorded session (integration test in `crates/slice/tests/sqlite_replay.rs`).
- [x] A v1 fixture migrates to v2 on open (column migration); payload-shape migration is lazy in `deserialize`.
- [x] WAL + `synchronous=FULL`; reopen preserves every acknowledged event. The process-kill test lands with the daemon (M1-07).
- [x] Secrets are redacted at write; the hash and marker survive export.

## Notes

- `rusqlite` bundled + WAL, one writer per session.
- Request hashes are blake3 hex (`orexis_core::hash_messages`), not `DefaultHasher`.
- `export_jsonl` is the first export surface; import lands with M1-07.
- Six unit tests + one integration test; clippy clean.
