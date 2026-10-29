# 027 — 100-cycle evolution gate and LTS stability guarantees (v1.0.0 LTS, M6)

- **Type:** task
- **Blocks:** v1.0.0 LTS
- **Status:** open
- **Claimed by:** —

## Question

What verification, calibration, and stability guarantees are required to declare Orexis v1.0.0 Long-Term Support (LTS) ready with autonomous self-evolution?

## Evidence

- `docs/orexis/PLAN.md` §5 & §5.1: M6 gate requires 100 autonomous self-mutation cycles with zero unrecoverable rollbacks.
- ADR-0015: Evolution tiers (tier 1: prompts/templates; tier 2: skills; tier 3: adapters; core sealed).
- `crates/usecases/src/evolution`: Validation ladder, rollback triggers, signed git ledger.
- Invariants: Sealed core signed + human; untrusted-content delimiting; zero telemetry.

## Proposed Resolution

1. **100-Cycle Autonomous Evolution Gate**:
   - Run a 100-cycle stress harness simulating autonomous prompt and skill mutations.
   - Inject deliberately broken mutations (syntax errors, infinite loops, capability regressions) and verify 100% rollback accuracy.
2. **LTS API & Format Stability**:
   - Freeze SQLite event schema migrations and JSON wire protocols for backwards compatibility.
   - Audit all security surfaces (untrusted content delimiters, permission rulesets, OS sandboxing).
3. **Formal Multi-Platform Audit**:
   - Run end-to-end verification across x86_64 Linux, AArch64 Linux, x86_64 macOS, and AArch64 macOS.
   - Verify that `/self` reports zero unauthorized network egress and zero telemetry.

## Follow-ups

- Publish `v1.0.0 LTS` release with signed binaries and desktop installers.
- Tag git branch `lts-v1.0` and publish long-term support maintenance policy.
