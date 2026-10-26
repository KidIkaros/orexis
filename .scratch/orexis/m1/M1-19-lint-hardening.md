# M1-19 — Boundary lint hardening

- **Blocked by:** —
- **Blocks:** M2, M3
- **Status:** resolved (devin, 2026-10-26)

## Goal

Enforce the Dependency Rule at the manifest level too: no `tokio` or adapter dependency inward, no provider-to-provider dependency. Record the daemon as an app layer.

## Acceptance

- [x] `check-boundaries.sh` fails if `tokio` appears in a `Cargo.toml` under `core`, `ports`, or `usecases`.
- [x] It fails if an adapter `Cargo.toml` depends on `orexis-usecases`.
- [x] It fails if one provider adapter depends on another (shared `provider-catalog` and `adapters-http` are allowed).
- [x] ADR-0006 defines `orexis-daemon` as an app/transport layer, not an adapter.
- [x] `AGENTS.md` states the daemon rule (rule 10).
- [x] A deliberate probe fails and the clean tree passes.

## Notes

- Probes: a `tokio` line in `usecases/Cargo.toml` and a provider-to-provider line in the Anthropic manifest both exit 1.
- `docs/orexis/seams.md` is generated and diffed by the gate.
