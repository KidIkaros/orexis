# 018 — Anthropic message-level cache breakpoints

- **Type:** feature
- **Blocks:** —
- **Status:** open
- **Claimed by:** —

## Question

The anthropic adapter expresses cache breakpoints on tool definitions
only (ADR-0018 manifest). The anthropic cache dialect can also carry
`cache_control` on the system prompt and on messages — for a coding
harness, the system prompt plus the project context are the stable
prefix, so message-level breakpoints are where the cache savings live.
Where do the breakpoints go (system, last context message, both), and
how is the breakpoint count budgeted (the wire allows a bounded number)?

## Evidence

- `adapters-provider-anthropic/tests/conformance.rs`:
  `message_level_cache_is_a_recorded_gap` ratchet asserts absence;
  closing the gap turns it red and forces a real fixture.
- PLAN §4.4: cache dialects are per-provider conditioning, paid once,
  centrally.
- PLAN invariant 3: the prompt prefix is stable within a session — the
  stable prefix is exactly what a cache breakpoint protects.

## Acceptance

- [ ] The adapter emits message-level `cache_control` at the decided
      positions under the decided budget.
- [ ] The conformance ratchet flips to a real fixture (expressed).
- [ ] Cache-hit behavior is verified against a live endpoint (manual,
      per the ADR-0016 real-peer pattern).

## Notes

The breakpoint positions and budget are the design question; the
emission is mechanical.
