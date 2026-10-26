# M4-02 — Audit trail

- **Blocked by:** M4-01
- **Blocks:** M4-04
- **Status:** resolved (devin, 2026-10-26)

## Goal

Privileged actions are durable and exportable.

## Acceptance

- [x] Approval decisions append `SessionEvent::ApprovalDecision` (tool, args, approved, always).
- [x] `ClientMsg::Audit` returns approval decisions plus shell/write/edit calls, checkpoints, verify, errors, and recovery.
- [x] Ordered by seq, capped at 500 entries; long errors truncated.
- [x] Test covers approve + always; deny shares the same path. *(A separate deny fixture is M4-04.)*

## Notes

- The event log is the source of truth; no separate audit DB.
- Redaction applies to audit text.
