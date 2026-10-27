# ADR-0018 — Provider conformance fixtures and the fidelity manifest

- **Status:** accepted
- **Date:** 2026-10-27
- **Deciders:** project owner, Devin
- **Related:** PLAN §4.4, §11 (provider matrix risk), ADR-0002, ADR-0003, ADR-0016, issue 013

## Context

ADR-0016 made native adapters on-demand, with a fidelity trigger: "a
per-dialect conformance fixture fails on the compatible endpoint: cache
breakpoints, thinking configuration, or tool-call fidelity the compatible
wire cannot express." But no fixture existed, so the trigger had no
evidence path, and the shipped profiles already declared capabilities the
adapters do not fully express:

- `openai.yaml` declares `thinking: { supported: true, param:
  reasoning_effort }`; the openai-compatible adapter never emits it.
- `anthropic.yaml` declares `thinking: { supported: true, param: thinking
  }`; the anthropic adapter never emits it.
- The anthropic cache dialect can carry `cache_control` on the system
  prompt and messages; the adapter emits it on tool definitions only.

The codec set (ADR-0003) already established the fixture pattern: sans-io
corpora run against the pure parser, no network, in CI.

## Decision

**A provider conformance fixture is a capability transcript: one fidelity
capability, a canonical set of model-visible messages, and the exact wire
body the adapter must produce — run against the shipped profile's
declarations, sans-io, in CI.**

Each provider adapter ships a `conformance` suite. Every capability a
shipped profile declares is in one of three states:

1. **Expressed** — the adapter's wire carries it; the fixture asserts the
   recorded body. Green.
2. **Gap (expressible-but-unimplemented)** — the wire can carry it and the
   profile declares it, but the adapter does not emit it. The ratchet
   asserts *absence*: closing the gap turns the ratchet red, forcing a
   real fixture and a manifest flip. A gap is an adapter fix (a follow-up
   ticket), **never** a native-adapter trigger.
3. **Wire-inexpressible** — the compatible wire cannot carry it at all.
   This is the ADR-0016 fidelity trigger: build the native adapter.

The state distinction is the point. ADR-0016's trigger fires only for
state 3; treating an adapter gap as a wire loss would schedule native
adapters for one-line fixes.

**First dialects: `open_ai` and `anthropic`** — the two shipped adapters
cover 45/50 hermes profiles and every shipped profile. Gemini native,
Bedrock, and `codex_responses` get suites when a trigger fires and an
adapter exists to test.

**Today's manifest** (no wire-inexpressible capability known):

| Dialect | Capability | State | Follow-up |
|---|---|---|---|
| `open_ai` | tool-call shape | expressed | — |
| `open_ai` | cache breakpoints | expressed (no-op: automatic caching, nothing on the wire) | — |
| `open_ai` | thinking config | expressed (ticket 017: `reasoning_effort` from the declared levels; Off emits the lowest — reasoning endpoints default on) | — |
| `anthropic` | tool-call shape | expressed | — |
| `anthropic` | cache breakpoints | expressed (system prefix + last tool; 2 of the wire's 4 breakpoints) | — |
| `anthropic` | thinking config | expressed (ticket 017: enabled flag + position-scaled budget, 1024 << (index + 1); Off/Auto omit — anthropic's default is off) | — |
| `anthropic` | message-level cache | expressed (ticket 018: the system prefix carries the breakpoint; the conversation tail waits for measured evidence) | — |

## Consequences

- The fidelity trigger has an evidence path: a wire-inexpressible fixture
  is the ADR-0016 trigger; an adapter gap is a ticket.
- Closing a gap is forced through the ratchet — the absence-assertion
  fails until a real fixture replaces it.
- New dialects inherit the template: canonical messages, capability
  fixtures, gap ratchets.
- Endpoint-side conformance (does the real endpoint honor what we send?)
  stays manual, per the ADR-0016 real-peer pattern.

## Alternatives considered

- **Fix the gaps in this ticket** — rejected: the ticket decides the
  fixture shape; adapter changes follow their own tickets with their own
  failing tests.
- **A shared cross-adapter conformance crate** — rejected: the suites test
  adapters, so they live with the adapters; a shared crate would need to
  depend on both adapters, inverting the dependency (adapters depend on
  the catalog, not the reverse).
- **Live-endpoint fixtures in CI** — rejected: network-dependent, slow,
  and flaky; the sans-io pattern from ADR-0003 applies.

## Follow-ups

- Ticket 017: emit thinking configuration per profile (`reasoning_effort`
  on the compatible wire, `thinking` on the anthropic wire). — resolved
  2026-10-27.
- Ticket 018: anthropic message-level cache breakpoints. — resolved
  2026-10-27: the system prefix carries the breakpoint; a conversation
  tail breakpoint waits for measured cache-hit evidence.
