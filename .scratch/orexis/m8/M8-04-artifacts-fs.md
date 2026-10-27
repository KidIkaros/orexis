# M8-04 — artifacts-fs: artifacts and leases

- **Blocked by:** M8-01
- **Blocks:** M8-09
- **Status:** open

## Goal

The blackboard adapter (ADR-0020): durable outputs (diff, log, generated
file) as files under a blackboard root, plus leases on workspace paths —
a lease names an owner and expires; a stale lease is stealable.

## Acceptance

- [ ] Put/get/list artifacts; artifacts are capped in size.
- [ ] Lease/acquire/steal-expired; a lease names its owner.
- [ ] The root is jailed (no path escapes).
