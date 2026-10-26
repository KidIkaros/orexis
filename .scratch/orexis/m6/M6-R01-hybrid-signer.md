# M6-R01 — Hybrid signer audit (experimental)

- **Type:** research/security
- **Blocked by:** —
- **Blocks:** — (not an M6 gate item)
- **Status:** open
- **Related:** ADR-0009, ADR-0015, `experiments/origin-hybrid-signer`

## Goal

Decide whether the Ed25519 + Falcon-1024 hybrid signer can become the
default evolution signer. It cannot until the audit and a new ADR.

## Current state

- Default: Ed25519 (`ed25519-dalek`) in `orexis-adapters-evolution-git`.
- Experiment: `experiments/origin-hybrid-signer`, not a workspace member,
  not built by default, uses `origin-crypto-sdk` 0.7.3 (path).
- The SDK says: experimental, not for production, no independent audit,
  custom PQC not audited, formats may change.

## Audit scope

- [ ] Independent review of hybrid composition and key binding.
- [ ] Domain separation and downgrade resistance (both signatures must verify).
- [ ] Key derivation, storage, rotation, and zeroization.
- [ ] Signature/key wire format stability and versioning.
- [ ] Cross-verification against known vectors and a second implementation.
- [ ] Dependency audit (`cargo-audit`, `cargo-deny`), license review.
- [ ] Performance: signature size, signing/verification cost, cache behavior.
- [ ] Failure modes: malformed lengths, partial signatures, panic paths.

## Decision

- [ ] Reject, keep experimental, or accept as opt-in.
- [ ] If accepted: new ADR, default remains Ed25519 unless explicitly changed.
- [ ] No production use before audit.

## Notes

- The `Signer` port is the seam; no corridor code changes are needed to
  switch signers.
- The experiment is isolated so a missing local SDK cannot break the default
  workspace build.
