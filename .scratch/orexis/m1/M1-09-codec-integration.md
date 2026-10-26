# M1-09 — Wire codecs into the loop

- **Blocked by:** M1-10, M1-11
- **Blocks:** M1-06
- **Status:** resolved (devin, 2026-10-26)

## Goal

The loop selects a codec, uses its prompt fragment in assembly, and decodes text-based provider output. Native providers bypass decoding. Decode errors become a reprompt, not a silent `Say`.

## Acceptance

- [x] `TurnDeps` carries the selected codec.
- [x] `assemble` uses `codec.prompt_fragment(tools)` — the loop computes it and passes it.
- [x] A text-based provider (`xml-manual`, `json`) completes a scripted tool-call task end to end.
- [x] A native provider bypasses `decode`.
- [x] Decode errors produce a `Reprompt` event and a bounded retry (two reprompts or `max_steps`), never a fatal parse.
- [x] Tests cover native bypass, XML text, JSON text, and malformed → reprompt.

## Notes

- `ProviderResponse.raw_text` is the decode input for non-native codecs; `ModelResponse` stores it.
- `SessionEvent::Reprompt` maps to a user message in `derive_messages`, so the next turn sees the correction.
- The loop assigns `call-{step}-{index}` when a codec leaves `call_id` empty.
- Four integration tests in `crates/slice/tests/codec_loop.rs`.
