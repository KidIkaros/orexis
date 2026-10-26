# M3-10 — Approval hub lifecycle

- **Blocked by:** —
- **Blocks:** M2
- **Status:** resolved (devin, 2026-10-26)

## Goal

Remove pending approvals on timeout and make broadcast lag visible instead of silently dropping approvals.

## Acceptance

- [x] A timed-out approval removes its sender from the pending map.
- [x] A dropped receiver removes its sender.
- [x] A lagged approval stream reports the skipped count to the client.
- [x] Tests: timeout cleanup, lagged error path.
