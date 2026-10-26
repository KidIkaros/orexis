# 013 — Per-dialect provider conformance fixtures

- **Type:** research
- **Blocks:** — (feeds ADR-0016's fidelity trigger)
- **Status:** open
- **Claimed by:** —

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

<filled when resolved>

## Follow-ups

<new tickets or fog this resolution creates>
