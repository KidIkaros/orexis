# ADR-0016 — Native adapters are on-demand, not milestone-scheduled

- **Status:** accepted
- **Date:** 2026-10-26
- **Deciders:** project owner, Devin
- **Related:** PLAN §4.4, §11 (provider matrix risk), ADR-0002, issue 009, M7

## Context

Ticket 009 asked which providers genuinely need a native adapter — Gemini
(`generateContent`), Bedrock/Vertex (SigV4/ADC), Copilot/Qwen (OAuth device
flow) — and when they land. The answer gates M7's scope.

Evidence from the shipped catalog:

- `hermes-providers.json` (50 profiles): 40 are `chat_completions` and 5 are
  `anthropic_messages` — **45/50 run on adapters that exist today**. The
  remainder need a different wire (4 `codex_responses`, 1 `bedrock_converse`)
  or non-key auth (OAuth device flow, `aws_sdk`, `vertex`, `external_process`).
- Gemini already ships as an OpenAI-compatible profile
  (`generativelanguage.googleapis.com/v1beta/openai`, M1-24); the M1 gate
  proved three live providers without a native Gemini wire.
- The profile schema already declares the native surface:
  `Dialect::Gemini`, `AuthKind::{OAuth, Adc, SigV4}` (ADR-0002). A native
  adapter is an adapter crate plus a composition arm — no schema change.
- No milestone gate requires a native adapter. M7's gate is protocol
  conformance (MCP client, ACP server, UTCP, A2A — one real peer each).

## Decision

**Native adapters are built on demand, when a trigger fires. M7 stays
protocols-only.**

Triggers (any one justifies building the adapter):

1. **Wire** — the user selects a profile with no compatible endpoint:
   `bedrock_converse`, Vertex `generateContent`, `codex_responses`.
2. **Fidelity** — a per-dialect conformance fixture fails on the compatible
   endpoint: cache breakpoints, thinking configuration, or tool-call fidelity
   the compatible wire cannot express.
3. **Auth** — the user selects a profile whose auth kind is unimplemented
   (Copilot/Qwen/MiniMax OAuth device flow, Bedrock SigV4, Vertex ADC). Auth
   is adapter-owned (ADR-0002) and separable from wire work; the wire may stay
   `openai_compatible`.

Until a trigger fires, a profile that names an unimplemented native wire fails
at composition with an error that says so and names the compatible path.

## Consequences

- M7 scope is unchanged; the frontier item unblocks it by scoping, not by
  scheduling adapter work.
- The stale composition error for `kind: gemini` ("lands in M1-07") is
  replaced by the decision text, with a regression test.
- Provider matrix maintenance (§11) keeps its mitigation: profiles as data,
  one compatible adapter, native only where the wire differs, conformance
  fixtures per dialect.
- `init` should make unimplemented auth kinds visible before selection
  (follow-up ticket).

## Alternatives considered

- **Build Gemini native in M7** — rejected: no gate needs it; the compatible
  endpoint works; M7 discipline stays protocols.
- **Build all native adapters in M7** — rejected: bloats the milestone and
  violates the smallest-correct-change rule.
- **Never build native adapters** — rejected: contradicts §4.4; the fidelity
  trigger will eventually fire (Gemini's native cache dialect is the likely
  first).

## Follow-ups

- Per-dialect provider conformance fixtures (feeds the fidelity trigger).
- `init`/`providers` surface which auth kinds are implemented.
- On-demand, unscheduled: OAuth device-flow adapter; Bedrock/Vertex native
  adapters.
