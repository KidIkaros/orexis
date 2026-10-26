# M1-03 — Anthropic and llama.cpp adapters

- **Blocked by:** M1-02
- **Blocks:** M1-08
- **Status:** resolved (devin, 2026-10-26) — implementation; live runs tracked by M1-08

## Goal

Add the two remaining native wires: Anthropic messages/tool blocks, and llama.cpp via `llama-server` with native tool calls.

## Acceptance

- [x] Anthropic streams text and `tool_use` blocks; the adapter normalizes them to `CallTool`.
- [x] Anthropic cache-control breakpoints are emitted when the profile declares them.
- [x] `llama-server` is spawned, health-checked over TCP, and killed with its process group (Unix).
- [x] llama.cpp native tool calls parse through the OpenAI-compatible adapter; `xml-manual` is selected when native is absent (ADR-0003).
- [x] The remote wire profile sends only standard fields; no llama.cpp-only fields.
- [x] Each adapter passes its mock conformance suite. Live runs are part of the M1-08 gate.

## Notes

- The HTTP transport moved to `orexis-adapters-http`; OpenAI and Anthropic share one seam.
- llama.cpp's provider is `OpenAiCompatibleProvider` with the `llamacpp` profile; this crate owns only process supervision, so adapters do not depend on each other.
- Twelve adapter tests across the two wires; clippy clean.
