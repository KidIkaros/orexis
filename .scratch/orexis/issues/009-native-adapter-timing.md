# 009 — Native adapter timing: Gemini / Bedrock / Vertex

- **Type:** research
- **Blocks:** M7
- **Status:** open
- **Claimed by:** —

## Question

Which providers genuinely need a native adapter — Gemini (`generateContent`), Bedrock/Vertex (SigV4/ADC), Copilot/Qwen (OAuth device flow) — and which land in M1 versus M7?

## Evidence

- Plan §4.4: native only where the wire differs.
- Hermes: 43 profiles; several are OAuth or cloud-IAM.
- M1 gate: OpenAI-compatible + Anthropic + llama.cpp live.
- Plan §14: native Gemini/Bedrock/Vertex timing is open.

## Resolution

<filled when resolved>

## Follow-ups

<new tickets or fog this resolution creates>
