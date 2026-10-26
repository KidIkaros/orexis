# M1-12 — Split the usecases module

- **Blocked by:** M1-09
- **Blocks:** —
- **Status:** resolved (devin, 2026-10-26)

## Goal

Split `crates/usecases/src/lib.rs` (500+ lines, five responsibilities) into focused modules, re-exported from `lib.rs`.

## Acceptance

- [x] No file exceeds 200 lines: `turn` 171, `compaction` 139, `context` 107, `actions` 71, `codec` 66, `replay` 53, `session` 14, `lib` 18.
- [x] Public API is unchanged; callers import from `orexis_usecases`.
- [x] All tests move with their module and stay green (7 unit tests).
- [x] No new dependencies.

## Notes

- Modules: `turn`, `actions`, `session`, `context`, `compaction`, `codec`, `replay`.
- `actions` and `session` are private helpers; the public surface is unchanged.
- Purely structural; no behaviour change.
