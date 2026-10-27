# 017 — Emit thinking configuration per profile

- **Type:** feature
- **Blocks:** —
- **Status:** open
- **Claimed by:** —

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

## Acceptance

- [ ] The adapters emit the profile's `thinking.param` with the requested
      level when thinking is requested; nothing when not.
- [ ] The conformance ratchets flip to real fixtures (expressed).
- [ ] The level source is decided: posture now, a CLI/profile override
      later, or both.

## Notes

The level source is the design question; the emission is mechanical.
