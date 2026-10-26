# ADR-0003 — Codec set and selection

- **Status:** accepted
- **Date:** 2026-10-25
- **Deciders:** project owner, Devin
- **Ticket:** `.scratch/orexis/issues/003-codec-set.md`

## Context

Orexis is model-agnostic: the same harness must drive frontier APIs with native tool calls, local models with trained XML dialects, small models that need grammar-constrained JSON, and unknown models that may need a tolerant fallback. Harness already proved four arms (`xml-manual`, `xml-native`, `json`, `lenient`); the question is what ships, how it is selected, and whether code-mode is a codec.

## Decision

**Four codecs ship:**

| Codec | What it is | Used for |
|---|---|---|
| `native` | Provider-native tool calls, normalized by the adapter | OpenAI, Anthropic, Gemini, llama-server with a native template |
| `xml-manual` | Client-side XML in the model's trained dialect | Local models without native tool calls (MiniCPM5) |
| `json` | Grammar-constrained flat JSON-schema actions | Small models, or providers with grammar support |
| `lenient` | Fenced ` ```action ` blocks, tolerant parser | Control and debugging only |

`native` is Harness's `xml-native` renamed: it is not XML, it is the native path with no prompt fragment.

**Code-mode is not a codec.** It is a program surface (`run_program`) that composes with any codec: the model emits a program as a tool call or text, and the harness executes it in the sandbox. It reduces round trips; it does not replace native tool calls.

**Selection is automatic**, in this order:

1. `native` if the model declares native tool-call support.
2. `xml-manual` if the model is a known XML-dialect model.
3. `json` if the provider supports grammar-constrained output.
4. `lenient` otherwise (with a warning).

A `--codec` flag overrides; a profile's model metadata may pin a codec.

**All codecs are sans-io and pure.** `decode(output, tools) -> Decoded` returns actions plus recoverable errors. Malformed action-looking fragments produce a `Reprompt`, never a fatal parse error.

**Multiple actions per output are allowed.** `step` orders them and executes sequentially; parallel execution is reserved for read-only tools.

**Fixtures.** Each codec ships a corpus of valid, malformed, and adversarial outputs. The suite is an M1 gate.

## Consequences

- One codec covers every native-tool provider; the adapter absorbs wire differences.
- Local models keep the arm that was probe-verified.
- Users do not choose an arm unless they want to; the default is correct by capability.
- `lenient` remains available for debugging but cannot become the default silently.
- Code-mode stays orthogonal, so it can ship later without touching the codec layer.
- Four codecs are a maintenance surface; fixtures and the selection policy are the guard.

## Alternatives considered

- **Ship only `native` and `xml-manual`** — fewer paths, but small models and grammar-capable providers lose their best arm.
- **Make code-mode a codec** — conflates parsing with execution; rejected.
- **Let the user pick the arm always** — Harness's model; it works for a probe tool but not for a product.
- **Auto-detect by probing the model** — network-dependent and slow; rejected in favour of catalog metadata plus a warning.
