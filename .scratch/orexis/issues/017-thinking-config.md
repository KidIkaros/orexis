# 017 — Emit thinking configuration per profile

- **Type:** feature
- **Blocks:** —
- **Status:** resolved
- **Claimed by:** devin, 2026-10-27

## Question

Both shipped adapters have the same conformance gap (ADR-0018): the
profiles declare thinking support (`openai.yaml`: `reasoning_effort`;
`anthropic.yaml`: `thinking` with levels), and the wires carry the
parameters, but neither adapter emits them. How does the adapter emit the
profile's thinking parameter — and where does the requested level come
from (posture, per ADR-0013's affect-modulated posture)?

## Evidence

- ADR-0018 manifest: `open_ai`/`anthropic` thinking config is a gap
  (expressible-but-unimplemented, state 2 — an adapter fix, not a native
  trigger).
- `adapters-provider-openai/tests/conformance.rs` and
  `adapters-provider-anthropic/tests/conformance.rs`: the ratchets assert
  absence; closing the gap turns them red and forces real fixtures.
- PLAN §4.4: per-provider conditioning is paid once, centrally — thinking
  effort is one of the conditioned dimensions.
- ADR-0013: posture carries thinking as a turn-boundary parameter.

## Resolution

**The `Provider` port gains the thinking request as a parameter; the
posture is its source; the adapters own the wire mapping.**

- `orexis_core::Thinking` gains `Level(usize)` — a positional index into
  the profile's declared `thinking.levels`, kept positional so it stays
  model-agnostic. The entity vocabulary (Off/Auto/On) is unchanged; the
  level is the harness-side refinement.
- `Provider::complete`/`stream` take `thinking: Thinking`. The turn loop
  carries it in `TurnDeps`; the daemon reads the entity snapshot's
  posture at the turn boundary (an affect-driven posture change lands on
  the next turn, never mid-session — ADR-0013's boundary rule).
- Adapter mapping (per-provider conditioning, paid once centrally):
  - `open_ai`: Auto omits the parameter (endpoint default); Off emits the
    **lowest** declared level — reasoning endpoints default on, so
    omission is not off; On emits the strongest; Level(i) clamps into the
    declared vocabulary.
  - `anthropic`: Off and Auto omit (anthropic's default is off); On is
    the strongest declared level; Level(i) enables thinking with a
    position-scaled budget, `1024 << (index + 1)`, bounded by the
    declared level count; `max_tokens` is raised above the budget (the
    wire requires it).
- The conformance ratchets flipped to real fixtures: both suites now
  assert expression (level indexing, Off/Auto omission, the budget
  scaling, and the max_tokens guard).
- The SSE parsers already tolerate thinking deltas (unknown event types
  fall through), so emitting thinking cannot break the loop.

**Level source decided: posture now; a CLI/session override later** — the
override's home is the init/settings surface (ticket 014, M9), not a
second config path bolted on here. The posture's thinking is Auto today
(`for_affect` never sets otherwise); the mechanism is wired so
affect-driven modulation flows when the entity work lands.

## Landed

- `core`: `Thinking::Level(usize)`.
- `ports`: `Provider::complete`/`stream` gain `thinking`.
- `adapters-provider-openai`, `adapters-provider-anthropic`: emission +
  conditioning; `request_body` takes the thinking level.
- `usecases`: `TurnDeps.thinking`; the loop passes it to the provider.
- `daemon`: `spawn_turn` reads the posture from the entity snapshot.
- Fakes and test providers gain the parameter (ignored or recorded).
- Conformance suites: ratchets flipped to real fixtures (ADR-0018
  manifest updated).

## Follow-ups

- A session/CLI thinking override (off/auto/on/level) belongs to the
  init/settings surface (ticket 014, M9).
- The affect→thinking mapping in `for_affect` is entity design (ADR-0013
  territory) and stays untouched here.
