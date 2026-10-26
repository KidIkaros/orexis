# 007 — Entity model: drives and affect

- **Type:** grilling
- **Blocks:** M5
- **Status:** resolved
- **Claimed by:** devin

## Question

Five drives or six? What affect dimensions does Orexis use, and how do drives and affect steer routing, risk, and retry without mutating the prompt mid-session?

## Evidence

- Kindred `src/soul.rs::Drives`: connection, curiosity, mastery, coherence, rest (five).
- Kindred design doc: six, adding growth. **Unresolved source-of-truth conflict.**
- MUE-X: seven drives; PAD affect.
- Plan §1.5: affect modulates routing/risk/retry, never the prompt mid-session.
- Plan invariant 18: workers share the entity's identity and memory.

## Resolution

**Six drives:** connection, curiosity, mastery, coherence, rest, growth.
Growth is the sixth; it is durable improvement, distinct from mastery (task
competence) and coherence (consistency). Connection is attunement, not
companionship.

**Affect is PAD:** valence, arousal, dominance, each in [-1, 1]. Moods are
derived display labels, not stored state.

**Drives propose; affect modulates posture.** Drives are slow pressures that
propose work at boundaries. Affect is fast and chooses a posture (model tier,
thinking, repair budget, verification, risk appetite, exploration) at a turn
boundary. Posture is not a Strategy.

**Invariants:** the prompt never changes mid-session; affect may add caution
but never weaken permissions; entity state is entity-level, and workers read
posture without mutating it. MUE-X's seven drives are not adopted without a
primary source.

ADR-0013.

## Follow-ups

- M5 spec and tickets: entity store, drives, affect/posture, memory lattice, identity distillation.
- M6 decision 008: evolution tiers and validation ladder, motivated by the growth drive.
- Revisit MUE-X's seventh drive only with its primary source.
