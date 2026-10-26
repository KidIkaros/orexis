# M6-01 — Corridor, ledger, seal, and signing

- **Blocked by:** —
- **Blocks:** M6-02 … M6-05
- **Status:** resolved (devin, 2026-10-26)

## Goal

A git-backed, append-only, signed evolution corridor with a sealed-set hash.

## Acceptance

- [x] Git-backed corridor; JSONL ledger with apply/revert events.
- [x] `Seal::compute` hashes the protected field map; callers supply the fields.
- [x] Ed25519 over the unsigned entry hash; candidate/seal/git hashes included.
- [x] Chain/signature verification; tamper fixture fails verify.
- [x] JSONL export/import round-trip. *(Ledger only; git history is not bundled.)*
- [x] Tests: apply/revert, tamper, restart, export/import; seal change in core.

## Notes

- Hybrid signatures wait for audit (ADR-0009).
- Composition wiring is M6-04; the adapter is constructed in tests only for now.
- Signing-key management (keyring/file) is a later ticket; tests use a seed.
- Import is ledger-only; the candidate/revert git commits are not bundled.
- The corridor is not the entity log; it records mutations, not identity.
