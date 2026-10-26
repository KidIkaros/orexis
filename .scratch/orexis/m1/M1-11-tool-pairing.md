# M1-11 — Tool-call pairing

- **Blocked by:** —
- **Blocks:** M1-09
- **Status:** resolved (devin, 2026-10-26)

## Goal

Carry a `call_id` from the model's tool call through the session event and the tool observation, and map it to OpenAI `tool_call_id` and Anthropic `tool_result.tool_use_id`.

## Acceptance

- [x] `Action::CallTool`, `SessionEvent::ToolCall`, and `SessionEvent::Observation` carry `call_id`.
- [x] `Message` carries an optional `tool_call_id` plus `tool_calls`; `derive_messages` pairs them.
- [x] OpenAI sends `role: "tool"` with `tool_call_id`; assistant messages carry `tool_calls`.
- [x] Anthropic sends `tool_result` with `tool_use_id`; assistant messages carry `tool_use` blocks.
- [x] Native adapters preserve the wire id (or synthesize `call_{index}` / `tool_{index}`); text codecs leave it empty and the loop assigns `call-{step}-{index}`.
- [x] Round-trip tests for both adapters and the derivation.

## Notes

- This is the change that makes multi-turn tool use correct against real APIs.
- `Message` now has `tool_call_id: Option<String>` and `tool_calls: Vec<ToolCall>`; all literals were updated.
