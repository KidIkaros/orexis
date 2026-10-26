# 003 — Codec set beyond the ported four

- **Type:** research
- **Blocks:** M1, M3
- **Status:** open
- **Claimed by:** —

## Question

Which codecs ship beyond Harness's four (`xml-manual`, `xml-native`, `json`, `lenient`), and is a native tool-calls codec required for remote providers? Is code-mode a codec or a separate program surface?

## Evidence

- Harness `crates/protocol/src/arm.rs`: four arms; `xml-manual` is production for the local model.
- Corpus: native tool calls for frontier models; code-mode as a program surface (opencode).
- Plan §4.3: `Codec` is sync and sans-io.

## Resolution

<filled when resolved>

## Follow-ups

<new tickets or fog this resolution creates>
