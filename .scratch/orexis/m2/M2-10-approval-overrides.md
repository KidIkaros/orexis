# M2-10 — Approval overrides as policy

- **Blocked by:** —
- **Blocks:** —
- **Status:** resolved (devin, 2026-10-26)

## Goal

"Always" currently adds the tool to a **daemon-wide** allow set inside `ApprovalHub`, a transport-layer policy override. Move the decision into the policy/use-case layer and scope it.

## Acceptance

- [x] `ApprovalOverrides` port; `MemoryOverrides` is the daemon implementation.
- [x] Session-scoped: `HubGate` carries the session id; `ApprovalRequest` carries `session_id`.
- [x] `ClientMsg::ClearOverrides` + `DaemonClient::clear_overrides`; appends an info event.
- [x] Tests: same session allows; second session still asks; clear restores the ask.
- [x] ADR-0011.

## Notes

- Architecture finding from the M2-02/03 audit: policy lives in the transport today.
- Overrides are in-memory only; restart clears them.
