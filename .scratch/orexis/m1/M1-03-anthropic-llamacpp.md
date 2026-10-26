# M1-03 — Anthropic and llama.cpp adapters

- **Blocked by:** M1-02
- **Blocks:** M1-08
- **Status:** open

## Goal

Add the two remaining native wires: Anthropic messages/tool blocks, and llama.cpp via `llama-server` with native tool calls.

## Acceptance

- [ ] Anthropic streams text and `tool_use` blocks; the adapter normalizes them to `ToolCall`.
- [ ] Anthropic cache-control breakpoints are emitted when the profile declares them.
- [ ] `llama-server` is spawned, health-checked, and killed with its process group.
- [ ] llama.cpp native tool calls parse; the `xml-manual` fallback is selected when native is absent.
- [ ] Remote wire profile strips llama.cpp-only fields.
- [ ] Each adapter passes its mock conformance suite and one live run.

## Notes

- Reuse Harness's subprocess supervision lessons (`PR_SET_PDEATHSIG`, log tail on failure).
- One generation at a time for the local provider; the daemon enforces it.
