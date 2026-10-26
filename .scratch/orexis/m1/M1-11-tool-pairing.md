# M1-11 — Tool-call pairing

- **Blocked by:** —
- **Blocks:** M1-09
- **Status:** open

## Goal

Carry a `call_id` from the model's tool call through the session event and the tool observation, and map it to OpenAI `tool_call_id` and Anthropic `tool_result.tool_use_id`.

## Acceptance

- [ ] `Action::CallTool`, `SessionEvent::ToolCall`, and `SessionEvent::Observation` carry `call_id`.
- [ ] `Message` carries an optional `tool_call_id`; `derive_messages` sets it for tool results.
- [ ] OpenAI sends `role: "tool"` with `tool_call_id`; Anthropic sends a `tool_result` block with `tool_use_id`.
- [ ] Native adapters preserve the wire id; text codecs synthesize one and the loop assigns it.
- [ ] Round-trip tests for both adapters.

## Notes

- This is the change that makes multi-turn tool use correct against real APIs.
