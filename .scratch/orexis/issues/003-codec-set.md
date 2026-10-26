# 003 — Codec set beyond the ported four

- **Type:** research
- **Blocks:** M1, M3
- **Status:** resolved
- **Claimed by:** devin, 2026-10-25

## Question

Which codecs ship beyond Harness's four (`xml-manual`, `xml-native`, `json`, `lenient`), and is a native tool-calls codec required for remote providers? Is code-mode a codec or a separate program surface?

## Evidence

- Harness `crates/protocol/src/arm.rs`: four arms; `xml-manual` is production for the local model.
- Corpus: native tool calls for frontier models; code-mode as a program surface (opencode).
- Plan §4.3: `Codec` is sync and sans-io.

## Resolution

**Decided 2026-10-25.** See `docs/orexis/adr/0003-codec-set.md`.

- **Four codecs ship:** `native`, `xml-manual`, `json`, `lenient`.
- **`native`** (renamed from Harness's `xml-native`) rides the provider's native tool-call path: OpenAI `tools`/`tool_calls`, Anthropic tool blocks, Gemini `functionCall`, llama-server's native template. The provider adapter normalizes every shape into `ToolCall`; the codec is shared.
- **`xml-manual`** stays the production arm for local models without native tool calls (MiniCPM5's trained XML dialect).
- **`json`** is the grammar-constrained arm: a flat JSON-schema action space, easiest for small models.
- **`lenient`** is a control/debug arm (fenced ` ```action ` blocks), never the default.
- **Code-mode is not a codec.** It is a program surface (`run_program`) that composes with any codec: the model emits a program as a tool call or text, and the harness executes it in the sandbox. It does not replace native tool calls.
- **Selection is automatic** by model capability — native if supported, `xml-manual` for a known XML-dialect model, `json` if grammar is available, `lenient` as the fallback — with a `--codec` override and a `codec` field in the profile's model metadata.
- **All codecs are sans-io and pure:** `decode(output, tools) -> Decoded`; malformed action-looking fragments produce `Reprompt`, never a fatal parse error.
- **Multiple actions per output are allowed**; `step` orders them and executes sequentially. Parallel execution is reserved for read-only tools.
- **Fixtures:** each codec ships a corpus of valid, malformed, and adversarial outputs; the suite is a gate for M1.

## Follow-ups

- ADR-0003 written.
- M1 implements `native`, `xml-manual`, `json`, and the selection policy; `lenient` is a test-only control.
- Code-mode is scheduled with M3 (program surface), not M1.
