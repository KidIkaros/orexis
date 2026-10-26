# M5-01 — Entity store and event model

- **Blocked by:** —
- **Blocks:** M5-02 … M5-06
- **Status:** resolved (devin, 2026-10-26)

## Goal

A sovereign, append-only entity log with projections and export/import.

## Acceptance

- [x] Entity event kinds: `EntityStart`, `DriveUpdate`, `AffectUpdate`, `DiaryAppend`, `TraitUpdate`, `MemoryWrite`, `MemoryForget`, `PostureChange`.
- [x] SQLite WAL store; `seq` + ULID event ids; JSON payloads; `entity.db` beside the session DB.
- [x] `EntityAppend`/`EntityRead`/`EntityFactory` ports; composition constructs the factory.
- [x] `project` derives drives/affect/posture/traits/memories/diary from the log.
- [x] Export/import JSONL round-trips.
- [x] Tests: append/projection, restart, export/import, redaction.

## Notes

- Separate from the session store; the entity is not a session.
- One entity per installation; workers share it.
- `Runtime.entity` is wired but no use case consumes it yet; M5-02/03 do.
- Composition is the only place `SqliteEntityFactory` is constructed.
