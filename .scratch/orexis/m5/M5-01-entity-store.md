# M5-01 — Entity store and event model

- **Blocked by:** —
- **Blocks:** M5-02 … M5-06
- **Status:** open

## Goal

A sovereign, append-only entity log with projections and export/import.

## Acceptance

- [ ] Entity event kinds: `EntityStart`, `DriveUpdate`, `AffectUpdate`, `DiaryAppend`, `TraitUpdate`, `MemoryWrite`, `MemoryForget`, `PostureChange`.
- [ ] SQLite WAL store under the Orexis home; `seq` + ULID; JSON payloads.
- [ ] `EntityAppend`/`EntityRead`/`EntityFactory` ports; adapter only in composition.
- [ ] Projections derive current drives/affect/traits/memories from the log.
- [ ] Export/import JSONL round-trips.
- [ ] Tests: append/read, restart, export/import, tamper detection for bad payloads.

## Notes

- Separate from the session store; the entity is not a session.
- One entity per installation; workers share it.
