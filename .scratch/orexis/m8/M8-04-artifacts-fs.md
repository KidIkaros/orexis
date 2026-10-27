# M8-04 — artifacts-fs: artifacts and leases

- **Blocked by:** M8-01
- **Blocks:** M8-09
- **Status:** resolved
- **Claimed by:** devin, 2026-10-27

## Goal

The blackboard adapter (ADR-0020): durable outputs (diff, log, generated
file) as files under a blackboard root, plus leases on workspace paths —
a lease names an owner and expires; a stale lease is stealable.

## Acceptance

- [ ] Put/get/list artifacts; artifacts are capped in size.
- [ ] Lease/acquire/steal-expired; a lease names its owner.
- [ ] The root is jailed (no path escapes).

## Landed

- `adapters-artifacts-fs` (new): `FsBlackboard` — artifacts as files
  under a jailed root (normalize + starts_with check; no escapes, no
  absolutes, no empties), capped at 8MB; leases in a write-through JSON
  (atomic rename) so a daemon restart does not forget who holds what;
  expired leases are stealable, live ones are not.
- Tests: round trip + jail, the cap, lease expiry/steal, reopen
  preserves leases.
