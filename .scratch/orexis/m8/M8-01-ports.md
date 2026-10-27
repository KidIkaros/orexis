# M8-01 — Ports: BoardStore, MessageStore, ArtifactStore, Provider::capacity

- **Blocked by:** —
- **Blocks:** M8-02 … M8-05
- **Status:** open

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

## Notes

Consumer-side: the traits live in `orexis-ports` because the use cases
consume them (rule 2).
