# M3-10 — Approval hub lifecycle

- **Blocked by:** —
- **Blocks:** M2
- **Status:** in progress

## Goal

Remove pending approvals on timeout and make broadcast lag visible instead of silently dropping approvals.

## Acceptance

- [x] A timed-out approval removes its sender from the pending map.
- [x] A dropped receiver removes its sender.
- [x] A lagged approval stream reports the skipped count to the client.
- [x] Tests: timeout cleanup, lagged error path.
