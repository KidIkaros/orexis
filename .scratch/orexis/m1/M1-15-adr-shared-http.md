# M1-15 — ADR: shared adapter infrastructure

- **Blocked by:** —
- **Blocks:** —
- **Status:** resolved (devin, 2026-10-26)

## Goal

Record why provider adapters may depend on `orexis-adapters-http`, or move the seam so they do not. The plan's rule says adapters depend on core + ports; `adapters-http` is an adapter-level shared crate.

## Acceptance

- [x] ADR written with context, decision, consequences, and alternatives.
- [x] The exception is recorded: shared infrastructure under `orexis-adapters-*`, external libraries only, no domain types, adapters only.
- [x] `AGENTS.md` updated with rule 9.

## Notes

- Moving HTTP into `ports` would leak a transport detail; the ADR records a narrow exception instead.
- Import-lint should allow `adapters-* -> adapters-http` and forbid `core/ports/usecases -> adapters-*`.
