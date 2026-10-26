# M1-04 — Codecs + selection + fixtures (ADR-0003)

- **Blocked by:** M1-02
- **Blocks:** M1-08
- **Status:** open

## Goal

Implement the four codecs (`native`, `xml-manual`, `json`, `lenient`), automatic selection from model capability, and the fixture suite.

## Acceptance

- [ ] Each codec decodes valid output into the expected actions.
- [ ] Malformed action-looking output yields `Reprompt`, never a fatal error.
- [ ] `native` emits no prompt fragment; `xml-manual` emits the trained XML docs.
- [ ] Selection: native → known XML dialect → grammar JSON → lenient, with `--codec` override.
- [ ] Multi-action output executes sequentially; read-only tools may run in parallel.
- [ ] Fixture suite covers valid, malformed, and adversarial outputs per codec.

## Notes

- Codecs are sans-io and pure; no provider or runtime types.
- The `lenient` arm is test-only and cannot become the silent default.
