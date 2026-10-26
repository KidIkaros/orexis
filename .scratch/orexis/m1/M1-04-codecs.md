# M1-04 — Codecs + selection + fixtures (ADR-0003)

- **Blocked by:** M1-02
- **Blocks:** M1-08
- **Status:** resolved (devin, 2026-10-26)

## Goal

Implement the four codecs (`native`, `xml-manual`, `json`, `lenient`), automatic selection from model capability, and the fixture suite.

## Acceptance

- [x] Each codec decodes valid output into the expected actions.
- [x] Malformed action-looking output returns recoverable `errors`; the loop turns them into `Reprompt` at M1-05.
- [x] `native` emits no prompt fragment; `xml-manual` emits the trained XML docs.
- [x] Selection: native → known XML dialect → grammar JSON → lenient, with a `--codec` override.
- [x] Multi-action output preserves order; the loop executes sequentially at M1-05 (parallel read-only is a loop concern).
- [x] Fixture suite covers valid, malformed, and adversarial outputs per codec.

## Notes

- `orexis-adapters-codec` depends only on core + ports; all four are sans-io and pure.
- `native` is a marker codec: the provider adapter already produced actions.
- `json` returns a grammar schema string for providers that support it.
- Ten codec tests plus the selection test; clippy clean.
