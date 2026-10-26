# M1-14 — Correct the compaction range

- **Blocked by:** —
- **Blocks:** —
- **Status:** resolved (devin, 2026-10-26)

## Goal

`plan_compaction` hardcodes `range_start_seq: 1`, which is wrong after the first compaction. Start the range after the previous compaction's end.

## Acceptance

- [x] The compaction event's range starts at the previous end + 1 (or 1 for the first).
- [x] A test with two compactions asserts the second range (3..4).
- [x] `derive_messages` behaviour is unchanged.

## Notes

- The range is audit metadata; derivation resets at the event position.
- `plan_compaction` scans backwards for the last `Compaction` and uses its `range_end_seq + 1`.
