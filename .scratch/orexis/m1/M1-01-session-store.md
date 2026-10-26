# M1-01 — SQLite session store (ADR-0001)

- **Blocked by:** —
- **Blocks:** M1-05, M1-06, M1-07, M1-08
- **Status:** open

## Goal

Implement the ADR-0001 store in `orexis-adapters-session-sqlite`: `session`, `event`, and `generation` tables; `SessionAppend`/`SessionRead`/`SessionFork`; lazy read-time migration; `orexis migrate`; request hashes with a real digest.

## Acceptance

- [ ] Append and read round-trip preserves `seq` order and `event_id`.
- [ ] Fork creates a new session with `parent_id` + `fork_seq` and copies the prefix.
- [ ] `derive_messages` + `verify_replay` pass over a recorded session.
- [ ] A v1 fixture migrates to v2 on read and rewrites on `migrate`.
- [ ] `kill -9` mid-append loses no acknowledged event; resume starts at the last `seq`.
- [ ] Secrets are redacted at write; the hash and marker survive export.

## Notes

- Use `rusqlite` (bundled) + WAL, one writer per session.
- Hash with blake3 or sha256, not `DefaultHasher`.
- JSONL export/import is a second surface over the same events.
