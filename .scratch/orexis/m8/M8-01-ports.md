# M8-01 — Ports: BoardStore, MessageStore, ArtifactStore, Provider::capacity

- **Blocked by:** —
- **Blocks:** M8-02 … M8-05
- **Status:** resolved
- **Claimed by:** devin, 2026-10-27

## Goal

The team domain's consumer-side ports (ADR-0020): `BoardStore` (tasks with
strategy, owner, status, `depends_on` edges), `MessageStore` (direct +
room messages, seq-ordered), `ArtifactStore` (durable outputs + leases
with owners and expiry), and `Provider::capacity()` (invariant 17 — the
scheduler asks the provider; default 1).

## Acceptance

- [ ] The ports are plain data in, plain data out; no adapter type
      crosses (rule 4).
- [ ] `Provider::capacity()` with a default of 1; the adapters return the
      profile's max_concurrency.
- [ ] Fakes for each port in `adapters-fake` (the testing pattern).

## Landed

- `orexis-core/src/team.rs` (new): `TaskStatus`, `BoardTask`, `BusMessage`,
  `Lease` — the data types live in core (core holds data types; ports hold
  traits).
- `orexis-ports`: `BoardStore` (create/get/list/ready/admit/start/complete/
  fail), `MessageStore` (post/read_since with worker filtering),
  `ArtifactStore` (put/get/list + lease/steal/lease_of);
  `Provider::capacity()` with a default of 1 — invariant 17 made real.
- `adapters-provider-openai`/`anthropic`: capacity returns the profile's
  max_concurrency.
- `adapters-fake`: `FakeBoard` (edges gate readiness, the cap refuses
  visibly, ordered transitions), `FakeBus` (seq-ordered, room+direct
  filtering), `FakeBlackboard` (jailed names, expiring + stealable
  leases).
- `adapters-fake/tests/team_fakes.rs`: the contracts the sqlite adapters
  must keep, encoded against the fakes.
- `daemon/tests/restart.rs`: the read/connect budgets raised to 120s —
  the gate chain runs the full suite at every level and child daemons
  starve; recovery takes 0.3s in isolation, so the budgets proved
  machine speed, not recovery.

## Notes

Consumer-side: the traits live in `orexis-ports` because the use cases
consume them (rule 2).
