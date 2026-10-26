# 007 — Entity model: drives and affect

- **Type:** grilling
- **Blocks:** M5
- **Status:** open
- **Claimed by:** —

## Question

Five drives or six? What affect dimensions does Orexis use, and how do drives and affect steer routing, risk, and retry without mutating the prompt mid-session?

## Evidence

- Kindred `src/soul.rs::Drives`: connection, curiosity, mastery, coherence, rest (five).
- Kindred design doc: six, adding growth. **Unresolved source-of-truth conflict.**
- MUE-X: seven drives; PAD affect.
- Plan §1.5: affect modulates routing/risk/retry, never the prompt mid-session.
- Plan invariant 18: workers share the entity's identity and memory.

## Resolution

<filled when resolved>

## Follow-ups

<new tickets or fog this resolution creates>
