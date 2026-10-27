# 013 — Per-dialect provider conformance fixtures

- **Type:** research
- **Blocks:** — (feeds ADR-0016's fidelity trigger)
- **Status:** resolved
- **Claimed by:** devin, 2026-10-27

## Question

What does a per-dialect provider conformance fixture look like, and which
dialects get one first? The fixture decides whether a compatible endpoint
loses fidelity the native wire would carry (cache breakpoints, thinking
config, tool-call shape).

## Evidence

- PLAN §11: provider matrix risk is mitigated by "conformance fixtures per
  dialect".
- ADR-0016 trigger 2: a native adapter is built when a fixture fails on the
  compatible endpoint.
- ADR-0003: codec set already uses sans-io fixtures as the pattern.

## Resolution

**A fixture is a capability transcript** (ADR-0018): one fidelity
capability, canonical model-visible messages, and the exact wire body the
adapter must produce — run against the shipped profile's declarations,
sans-io, in CI. Three states: **expressed** (fixture asserts the body),
**gap** (expressible-but-unimplemented — a ratchet asserting absence; an
adapter fix, never a native trigger), **wire-inexpressible** (the
ADR-0016 fidelity trigger). The state distinction keeps one-line adapter
fixes from scheduling native adapters.

**First dialects: `open_ai` and `anthropic`** — the two shipped adapters
cover 45/50 hermes profiles and every shipped profile.

The first suites found the manifest (no wire-inexpressible capability
known today): thinking config is a gap on both wires (profiles declare
`reasoning_effort`/`thinking`; neither adapter emits it), and
message-level cache is a gap on the anthropic wire (tools-only today).
Open_ai cache is a documented no-op (automatic caching).

## Landed

- `adapters-provider-openai/tests/conformance.rs`: tool-call shape,
  open_ai cache no-op, thinking gap ratchet.
- `adapters-provider-anthropic/tests/conformance.rs`: tool-call shape,
  tools cache breakpoint, thinking gap ratchet, message-level cache gap
  ratchet.
- ADR-0018 records the design, states, manifest, and first dialects.

## Follow-ups

- Ticket 017: thinking configuration per profile (both adapters; the
  level source — posture vs override — is the design question).
- Ticket 018: anthropic message-level cache breakpoints (positions and
  budget are the design question).
