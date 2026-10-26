# ADR-0013 — Entity model: six drives and PAD affect

- **Status:** accepted
- **Date:** 2026-10-26
- **Deciders:** project owner, Devin
- **Related:** PLAN §1.5, invariant 18, ticket 007, M5

## Context

The entity is the persistent identity layer: identity, drives, affect, and
memory. Ticket 007 asked two questions: five drives or six, and which affect
dimensions Orexis uses.

Evidence:

- Kindred `src/soul.rs::Drives` has **five**: connection, curiosity, mastery,
  coherence, rest.
- Kindred `DESIGN.md` v0.3 says it **added a sixth drive, growth**, but its
  drive table still lists five. The design intent and the source disagree.
- Kindred's glial layer modulates drives with six discrete emotions (joy,
  grief, anxiety, fatigue, contentment, excitement), clamped to [0.3, 1.5].
- Harness `crates/core/src/affect.rs` uses a two-axis affect (frustration,
  confidence), deterministic and bounded, to modulate strategy: codec
  escalation, sampling, repair budget. It is never persisted and never
  mutates the prompt.
- MUE-X is described as seven drives with PAD, but no MUE-X source is present
  in this workspace; the claim is second-hand.
- PLAN §1.5: affect modulates routing, risk, and retry — **never the prompt
  mid-session**. Invariant 18: workers share the entity's identity and memory.

## Decision

### Six drives

Orexis uses **six** drives, each a slow pressure in `[0, 1]` with a baseline,
decay, and satisfaction:

| Drive | What it wants | Rises when | Falls when |
|---|---|---|---|
| **Connection** | Attunement to the user's intent and working relationship | Ambiguity, long absence, a handoff | The user and agent are aligned |
| **Curiosity** | To understand the unknown | Unfamiliar code, an open question | The question is answered |
| **Mastery** | Competence in the current task | A hard problem, a failing test | The task is completed well |
| **Coherence** | Internal consistency and integration | Contradictory evidence, a broken invariant | Reflection resolves it |
| **Rest** | Consolidation instead of thrash | Long turns, repeated retries | Quiet, a checkpoint, a stop |
| **Growth** | Durable improvement, not just this task | Repeated patterns, a better approach | The improvement is accepted |

**Growth is the sixth.** It is distinct from mastery (competence *now*) and
coherence (consistency *now*): growth is the trajectory. It is the drive that
proposes evolution candidates in M6.

Connection is **attunement**, not companionship. Orexis is a coding harness;
the drive keeps the agent aligned with the user's intent, not bonded to them.

### PAD affect

Affect is three continuous dimensions, each in `[-1, 1]`:

| Dimension | Means | Rises on | Falls on |
|---|---|---|---|
| **Valence** | Outcome quality | Success, a passing verify | Failure, a rejected approval |
| **Arousal** | Urgency and load | Retries, budget pressure, errors | Completion, quiet |
| **Dominance** | Control and confidence | A clear plan, a successful tool | Repeated failure, ambiguity |

Moods (`calm`, `focused`, `strained`, `cautious`, `curious`, `restless`) are
**derived display labels**, not stored state.

### Drives propose; affect modulates posture

- **Drives** are slow. They propose work at turn/session boundaries: curiosity
  → explore, mastery → build, coherence → refactor, rest → consolidate/stop,
  growth → evolution candidate, connection → check-in/summary.
- **Affect** is fast. It chooses a **posture** at a turn boundary: model tier,
  thinking mode, repair budget, verification level, risk appetite, exploration
  level.
- **Posture is not a Strategy.** A Strategy is a use case (`plan`, `build`,
  `review`, `mine`). Posture is execution parameters.
- **The prompt never changes mid-session.** No system-prompt or context
  mutation for drives or affect; identity distillation happens at session
  boundaries. This preserves the prompt-cache invariant.
- **Affect may add caution, never remove safety.** It can raise verification,
  lower risk appetite, or reduce exploration. It cannot turn a `deny` into
  `allow`, bypass plan mode, or weaken the permission policy.
- **Entity-level state.** Drives and affect belong to the entity. Workers share
  identity and memory and read the entity's posture; they do not mutate it.

### MUE-X

No MUE-X primary source is available. Orexis adopts six drives and PAD now. A
seventh drive requires evidence from the MUE-X source and a new ADR; it is not
adopted on a second-hand claim.

## Consequences

- M5 stores six drive values, three affect values, baselines, and history.
- M6's evolution proposals are motivated by the growth drive, gated by the
  corridor.
- `/self` shows drives, PAD values, and derived mood labels.
- No prompt-cache break from entity state.
- The Kindred source/design conflict is resolved in Orexis's favor of six, with
  growth defined rather than implied.

## Alternatives considered

- **Five drives** — rejected: drops the differentiator and the design-doc
  intent; mastery and growth are not the same pressure.
- **Seven drives** — rejected: no primary source; adding a drive without
  evidence is exactly the kind of speculative model the plan warns against.
- **Discrete emotions as canonical** — rejected: PAD is compact, continuous,
  and maps to posture; Kindred's six emotions can be derived labels later.
- **Harness's two axes** — kept as an implementation hint for update rules,
  not as the model; it is a projection of PAD.
