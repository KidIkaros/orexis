# M6-01 — Corridor, ledger, seal, and signing

- **Blocked by:** —
- **Blocks:** M6-02 … M6-05
- **Status:** open

## Goal

A git-backed, append-only, signed evolution corridor with a sealed-set hash.

## Acceptance

- [ ] `.orexis/evolution` git repo; JSONL ledger with apply/revert events.
- [ ] `Seal` hashes permissions, sandbox, provider/model, tools, codec/protocol, secrets, telemetry, safety floor, identity policy.
- [ ] Ed25519 signature over canonical ledger entry; candidate/git hashes included.
- [ ] Chain/signature verification; tamper blocks apply.
- [ ] Export/import of the corridor.
- [ ] Tests: apply/revert, seal change, tamper, restart.

## Notes

- Hybrid signatures wait for audit (ADR-0009).
- The corridor is not the entity log; it records mutations, not identity.
