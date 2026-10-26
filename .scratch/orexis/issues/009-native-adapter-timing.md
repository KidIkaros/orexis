# 009 — Native adapter timing: Gemini / Bedrock / Vertex

- **Type:** research
- **Blocks:** M7
- **Status:** resolved (devin, 2026-10-26) — ADR-0016
- **Claimed by:** devin, 2026-10-26

## Question

Which providers genuinely need a native adapter — Gemini (`generateContent`), Bedrock/Vertex (SigV4/ADC), Copilot/Qwen (OAuth device flow) — and which land in M1 versus M7?

## Evidence

- Plan §4.4: native only where the wire differs.
- Hermes: 43 profiles; several are OAuth or cloud-IAM.
- M1 gate: OpenAI-compatible + Anthropic + llama.cpp live.
- Plan §14: native Gemini/Bedrock/Vertex timing is open.

## Resolution

Native adapters are on-demand, not milestone-scheduled; M7 stays
protocols-only (ADR-0016). Catalog data: 45/50 Hermes profiles run on
adapters that exist today; Gemini already ships as an OpenAI-compatible
profile (M1-24); the schema already declares the native surface
(`Dialect::Gemini`, `AuthKind::{OAuth, Adc, SigV4}`).

Triggers: wire (no compatible endpoint), fidelity (a per-dialect fixture
fails on the compatible endpoint), auth (unimplemented auth kind; separable
from wire work). The stale composition error for `kind: gemini` now states
this and names the compatible path — regression test in
`composition/tests/build.rs`.

## Follow-ups

- [013](013-provider-wire-conformance-fixtures.md) — per-dialect provider
  conformance fixtures; feeds the fidelity trigger.
- [014](014-init-auth-kind-visibility.md) — `init`/`providers` surface which
  auth kinds are implemented, before selection.
- On-demand, unscheduled: OAuth device-flow adapter; Bedrock/Vertex native
  adapters.
- Plan hygiene: §14's open list still names items resolved by ADRs
  0001/0003/0004/0013/0015; prune at the next map revision (M7).
