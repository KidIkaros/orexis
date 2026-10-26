# M1-09 — Wire codecs into the loop

- **Blocked by:** M1-10, M1-11
- **Blocks:** M1-06
- **Status:** open

## Goal

The loop selects a codec, uses its prompt fragment in assembly, and decodes text-based provider output. Native providers bypass decoding. Decode errors become a reprompt, not a silent `Say`.

## Acceptance

- [ ] `TurnDeps` carries the selected codec and its capabilities.
- [ ] `assemble` uses `codec.prompt_fragment(tools)` instead of a generic tool list.
- [ ] A text-based provider (`xml-manual`, `json`, `lenient`) completes a scripted tool-call task end to end.
- [ ] A native provider bypasses `decode`.
- [ ] Decode errors produce a `Reprompt`/`Error` event and a bounded retry, never a fatal parse.
- [ ] Tests cover native bypass, xml text, json text, and malformed → reprompt.

## Notes

- The provider returns raw text alongside native actions (`ProviderResponse`), so the loop can choose.
- Codec selection stays a use case; the codec instance is injected by composition.
