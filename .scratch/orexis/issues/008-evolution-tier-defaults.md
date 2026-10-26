# 008 — Evolution tier defaults and validation ladder

- **Type:** grilling
- **Blocks:** M6
- **Status:** resolved
- **Claimed by:** devin

## Question

What are the tier defaults (`off`, `skills`, `quarantine`, `core`), the validation ladder, the cadence, and the rollback triggers for evolution?

## Evidence

- Plan invariant 7: tiered, git-backed, sealed, signed; quarantine never auto-applies Python.
- MUE-X: six mutation strategies, immune system, protected files, rollback, dedup, size limits.
- Kindred: `evolve.rs`, `soul_corridor.rs`, `immune.rs`.
- Harness: genes ledger + seal; no mutation loop.

## Resolution

**Defaults:** `off` on a fresh install. `skills` is the first opt-in and may
auto-apply after the full ladder. `quarantine` is stage-only. `core` is
never automatic; it needs a signed candidate and human approval. Every
candidate is quarantined first.

**Ladder:** schema -> seal -> static safety -> deterministic tests ->
behavioral evaluation (paired, noise band) -> 24h quarantine soak -> signed
git apply -> monitor/rollback.

**Sealed:** permissions, sandbox, provider/model, tools, codec/protocol,
secrets, telemetry, safety floor, identity policy. Mutation is data only:
prompt/strategy fragments, gene parameters, skill content, recall/reflection
policy. No executable content auto-applies.

**Cadence:** session boundaries only; one proposal per session/day; one
skills apply/day; core on human approval; 100 accelerated cycles for the gate.

**Rollback:** automatic on safety/chain/signature/unrecoverable failure; auto
for skills on regression; human for core. Git revert + ledger event.

ADR-0015.

## Follow-ups

- M6 spec and tickets: corridor/ledger, proposals/quarantine, validation, tiers/apply/rollback, 100-cycle gate.
- Evolution calibration after 100 cycles remains fog.
