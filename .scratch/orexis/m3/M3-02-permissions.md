# M3-02 — Permission ruleset and plan gate

- **Blocked by:** —
- **Blocks:** M3-03, M3-08
- **Status:** resolved (devin, 2026-10-26)

## Goal

A `deny > ask > allow` ruleset with per-tool latitude, plan mode read-only, and approval-required tools that block.

## Acceptance

- [x] Each tool declares `auto`, `approval_required`, or `forbidden` through the policy defaults.
- [x] Plan mode forbids writes and shell, regardless of the ruleset.
- [x] A deny rule beats an ask rule beats an allow rule.
- [x] Approval-required tools block until approved; denial returns an error observation.
- [x] The daemon's `Approve` path is wired to the hub; a 300 s timeout prevents a hung turn.
- [x] Adversarial tests: plan-mode write, denied shell, approval round-trip.

## Notes

- `PermissionPolicy` lives in `orexis-core`; `ApprovalGate` is a port; `GatedTools` wraps a tool executor.
- `ApprovalHub` broadcasts `ApprovalRequest` to connected clients; `Approve { id, yes }` resolves it.
- Approvals are currently global to the daemon, not per-session; a session-scoped bus is a later refinement.
