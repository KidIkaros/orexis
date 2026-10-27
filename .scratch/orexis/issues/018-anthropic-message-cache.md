# 018 — Anthropic message-level cache breakpoints

- **Type:** feature
- **Blocks:** —
- **Status:** resolved
- **Claimed by:** devin, 2026-10-27

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

## Resolution

**The breakpoint goes on the system prefix; the conversation tail waits
for measured evidence.**

Our assembly puts the system prompt, tool docs, and project context into
one system message — the stable prefix invariant 3 protects. That is
where a breakpoint earns its keep, so when `cache.breakpoints` is on the
system travels as one text block with `cache_control: ephemeral`; with
breakpoints off (openrouter-anthropic) it stays a plain string. The
conversation tail gets no breakpoint: it changes every turn, and a
moving breakpoint's write cost (cache re-written at a new position each
turn) needs measured cache-hit evidence before it earns its place. Two
of the wire's four breakpoints are used (system + last tool), leaving
headroom.

## Landed

- `adapters-provider-anthropic`: the system field carries the breakpoint
  when the profile asks for it.
- Conformance suite: the ratchet flipped to
  `system_prefix_carries_the_cache_breakpoint`; new fixture
  `breakpoints_off_leave_the_system_a_plain_string` (openrouter-anthropic
  profile) pins the no-breakpoint wire form.
- ADR-0018 manifest: message-level cache is expressed.

## Follow-ups

- A conversation-tail breakpoint (incremental history caching) waits for
  measured evidence from a live endpoint — part of the M6/M9 calibration
  work, not a fixture-level decision.
