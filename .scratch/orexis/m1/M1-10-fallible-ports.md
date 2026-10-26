# M1-10 — Fallible provider and tool ports

- **Blocked by:** —
- **Blocks:** M1-09, M1-11, M1-06
- **Status:** resolved (devin, 2026-10-26)

## Goal

Make `Provider` and `ToolExecutor` fallible, so transport and tool failures are distinguishable from model output. Add `ProviderResponse` (actions + raw text) and a durable `Error` event.

## Acceptance

- [x] `Provider::complete -> PortResult<ProviderResponse>`; `ProviderResponse` carries `actions` and `raw_text`.
- [x] `ToolExecutor::execute -> PortResult<Observation>`.
- [x] A transport failure returns `Err`, not a fake `Say`/`Done` (OpenAI and Anthropic tests).
- [x] A tool failure returns `Err` and appends an `Error` event (slice integration test).
- [x] `SessionEvent::Error` is persisted, ignored by `derive_messages`, and redacted.
- [x] All adapters and the fake compile and pass their tests.

## Notes

- `PortError` stays a string for now; typed errors are a later refinement.
- `ProviderResponse.raw_text` is the hook M1-09 needs to decode text-based providers.
- The loop appends `Error` before returning `Err`, so the failure is visible in replay.
