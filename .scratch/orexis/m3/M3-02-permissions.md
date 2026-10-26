# M3-02 — Permission ruleset and plan gate

- **Blocked by:** —
- **Blocks:** M3-03, M3-08
- **Status:** open

## Goal

A `deny > ask > allow` ruleset with per-tool latitude, plan mode read-only, and approval-required tools that block.

## Acceptance

- [ ] Each tool declares `auto`, `approval_required`, or `forbidden`.
- [ ] Plan mode forbids writes and shell, regardless of the ruleset.
- [ ] A deny rule beats an ask rule beats an allow rule.
- [ ] Approval-required tools block until approved; denial returns an error observation.
- [ ] The daemon's `Approve` path is wired to the gate.
- [ ] Adversarial tests: plan-mode write, denied shell, approval timeout.
