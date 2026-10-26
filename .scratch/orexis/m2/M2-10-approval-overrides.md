# M2-10 — Approval overrides as policy

- **Blocked by:** —
- **Blocks:** —
- **Status:** open

## Goal

"Always" currently adds the tool to a **daemon-wide** allow set inside `ApprovalHub`, a transport-layer policy override. Move the decision into the policy/use-case layer and scope it.

## Acceptance

- [ ] An override is represented as data (tool, scope) in the inner layer, not a daemon-local set.
- [ ] Scope is explicit: session or daemon; per-session is the default.
- [ ] Revocation path exists (clear overrides / session end).
- [ ] Daemon tests: always allows within scope, not across sessions.
- [ ] ADR records the decision.

## Notes

- Architecture finding from the M2-02/03 audit: policy lives in the transport today.
- Current behaviour is single-user acceptable but not the target model.
