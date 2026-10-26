# M1-14 — Correct the compaction range

- **Blocked by:** —
- **Blocks:** —
- **Status:** open

## Goal

`plan_compaction` hardcodes `range_start_seq: 1`, which is wrong after the first compaction. Start the range after the previous compaction's end.

## Acceptance

- [ ] The compaction event's range starts at the previous end + 1 (or 1 for the first).
- [ ] A test with two compactions asserts the second range.
- [ ] `derive_messages` behaviour is unchanged.

## Notes

- The range is audit metadata; derivation resets at the event position.
