# M1-12 — Split the usecases module

- **Blocked by:** M1-09
- **Blocks:** —
- **Status:** open

## Goal

Split `crates/usecases/src/lib.rs` (457 lines, 12 public items) into `loop.rs`, `context.rs`, `compaction.rs`, `codec.rs`, and `replay.rs`, re-exported from `lib.rs`.

## Acceptance

- [ ] No file exceeds 200 lines.
- [ ] Public API is unchanged; callers import from `orexis_usecases`.
- [ ] All tests move with their module and stay green.
- [ ] No new dependencies.

## Notes

- Purely structural; no behaviour change.
