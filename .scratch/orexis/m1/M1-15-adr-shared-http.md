# M1-15 — ADR: shared adapter infrastructure

- **Blocked by:** —
- **Blocks:** —
- **Status:** open

## Goal

Record why provider adapters may depend on `orexis-adapters-http`, or move the seam so they do not. The plan's rule says adapters depend on core + ports; `adapters-http` is an adapter-level shared crate.

## Acceptance

- [ ] ADR written with context, decision, consequences, and alternatives.
- [ ] Either the exception is recorded, or `Transport` moves to a neutral crate with the same dependency direction.
- [ ] `AGENTS.md` is updated if the rule changes.

## Notes

- Moving HTTP into `ports` would leak a transport detail; the ADR likely records a narrow exception.
