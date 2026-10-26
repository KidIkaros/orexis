# M4-02 — Audit trail

- **Blocked by:** M4-01
- **Blocks:** M4-04
- **Status:** open

## Goal

Privileged actions are durable and exportable.

## Acceptance

- [ ] Approval decisions append a durable event (tool, args, approved, always).
- [ ] `ClientMsg::Audit` returns approval decisions plus shell/write/edit calls, checkpoints, verify, and errors.
- [ ] Audit export is ordered by seq and capped.
- [ ] Tests cover approve, deny, and always.

## Notes

- The event log is the source of truth; no separate audit DB.
- Redaction applies to audit text.
