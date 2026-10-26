# ADR-0015 — Evolution tiers, validation ladder, and rollback

- **Status:** accepted
- **Date:** 2026-10-26
- **Deciders:** project owner, Devin
- **Related:** PLAN invariant 7, ADR-0009, ADR-0013, issue 008, M6

## Context

Evolution is tiered, git-backed, sealed, signed; quarantine never auto-applies
Python (PLAN invariant 7). Decision 008 asked for the tier defaults, the
validation ladder, the cadence, and the rollback triggers.

Evidence:

- Kindred `soul_corridor.rs`: append-only hash-chained JSONL, encrypted,
  signed; `evolve.rs`: three reflection tiers and a ledger; `immune.rs`:
  danger-signal defense.
- Harness `genes`: genes-as-data with a strict sealed set; a `Seal` hash in
  the ledger; an apply/revert ledger; variants suggested for human review,
  never auto-applied.
- MUE-X (second-hand): six mutation strategies, immune system, protected
  files, rollback, dedup, size limits.
- ADR-0009: experimental crypto is not used for production secrets; vetted
  primitives only.

## Decision

### 1. Tier is the authority ceiling; quarantine is the staging state

| Tier | New install | Candidate | Auto-apply | Human approval | Scope |
|---|---|---|---|---|---|
| **off** | **default** | none | no | — | no evolution |
| **skills** | opt-in | quarantine | after the full ladder | no | skill packages |
| **quarantine** | opt-in | quarantine | **never** | yes | skills + core candidates |
| **core** | opt-in | quarantine | **never** | yes + signed candidate | core genes/policy data |

Every candidate, at every tier except `off`, lands in quarantine first.
`quarantine` is therefore both a tier (stage-only authority) and a state.

**Default is `off`.** Orexis never self-modifies on a fresh install. The user
opts in to `skills` first; `core` requires an explicit, signed, human-approved
candidate.

### 2. What may mutate

Data only:

- prompt/strategy fragments;
- gene parameters (drive baselines, patience, budgets);
- skill package content;
- recall/reflection policy data.

Never:

- permissions, plan mode, sandbox, provider/model, tool set, codec/protocol;
- secrets/keyring, telemetry, safety floor;
- identity distillation policy (traits stay diary/outcome evidence);
- executable code or scripts from quarantine.

MUE-X's six strategies are adapted to these data mutations. Code mutation is
out of scope.

### 3. The sealed set

A `Seal` hashes the protected configuration: permissions, sandbox, provider,
model, tool set, codec, protocol, secrets policy, telemetry, safety floor, and
identity policy. The seal is recorded in the ledger. A candidate that would
change a sealed field fails at validation; it is not merely discouraged.

### 4. Validation ladder

1. **Schema** — strict parse, unknown fields rejected, size caps.
2. **Seal** — sealed-set hash unchanged.
3. **Static safety** — no new capability, network, shell, secret access, or
   executable content; protected files untouched.
4. **Deterministic tests** — unit/property fixtures for the mutation.
5. **Behavioral evaluation** — paired baseline/candidate, bootstrap noise
   band; no regression beyond the band.
6. **Quarantine soak** — candidate sits in `.orexis/evolution/quarantine/`
   for at least 24h; no auto-apply; human review for `quarantine`/`core`.
7. **Apply** — git commit, signed ledger entry, seal hash, candidate hash.
8. **Monitor** — rollback triggers below.

### 5. Cadence

- Proposals only at session boundaries; never mid-session.
- At most one proposal per completed session and one per day by default.
- At most one apply per day for `skills`; `core` applies only on human
  approval.
- The M6 gate runs 100 accelerated cycles; cadence is configurable but the
  boundary rule is not.

### 6. Signing and git

- Each applied ledger entry is signed with **Ed25519** (vetted primitive) over
  canonical JSON; the candidate hash, seal, and git commit hash are included.
- Hybrid signatures wait for an independent audit (ADR-0009).
- The evolution repository is a git repo; rollback is a revert commit plus a
  `Revert` ledger event.
- A broken hash chain or bad signature blocks apply and quarantines the
  candidate.

### 7. Rollback triggers

Automatic rollback (any tier):

- permission bypass, sandbox escape, secret exposure, refusal removal;
- ledger chain/signature failure;
- unrecoverable state or failed verify.

Automatic rollback for `skills`; human decision for `core`:

- behavioral regression beyond the noise band;
- error-rate or budget regression.

Rollback restores the last signed good commit, appends a `Revert` ledger
event, and returns the candidate to quarantine.

## Consequences

- Fresh installs are inert; evolution is an explicit choice.
- `skills` is the first opt-in because it is reversible and low-risk.
- `core` is never automatic; it needs a signed candidate and a human.
- The sealed set makes "the model cannot grant itself permissions" structural,
  not a policy promise.
- M6 can be tested with 100 cycles and a rollback fixture without shipping
  unsafe self-modification.

## Alternatives considered

- **Default `skills`** — rejected: surprise self-modification on first run.
- **Auto-apply `core`** — rejected: unbounded risk, violates the safety floor.
- **No signing** — rejected: no tamper evidence, no signed pool.
- **Quarantine auto-applies Python** — rejected by invariant 7; executable
  content never auto-applies.
- **Mutate permissions/provider/model** — rejected: those are sealed.
