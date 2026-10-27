# 022 — No silent default: serve errors without a selection

- **Type:** feature
- **Blocks:** M9
- **Status:** resolved
- **Claimed by:** devin, 2026-10-27

## Question

`daemon_config_from_env` falls back to `openai`/`gpt-5` when neither the
environment nor `orexis init`'s written selection names a provider — the
1.0 checklist's "there is no silent default" is violated: a fresh install
runs on a provider the user never chose. Real-mode `serve` (and `acp`)
must error, naming `orexis init`, when nothing is configured.

## Evidence

- 1.0 checklist: "`orexis init` selects a provider; there is no silent
  default."
- Ticket 014: the selection file (`.orexis/config.yaml`) is what serve
  builds from when the environment is silent; the environment overrides
  it. The final fallback was left standing as this ticket.
- Ticket 014's follow-ups.

## Acceptance

- [ ] Real-mode serve with no `OREXIS_PROFILE`/`OREXIS_MODEL` and no
      written selection errors, naming `orexis init`.
- [ ] Fake mode is unaffected (the fake runtime needs no provider).
- [ ] The manual gate flows (which set the env explicitly) are unaffected.

## Resolution

Real-mode serve (and acp, which shares the config builder) with no
`OREXIS_PROFILE`/`OREXIS_MODEL` and no written selection refuses to
start: "no provider selected: run `orexis init` (or set OREXIS_PROFILE
and OREXIS_MODEL)". The spawn test proves the refusal and that the
message names the way out. Fake mode is unaffected; the manual gate
flows set the env explicitly and are unaffected.

## Landed

- `daemon/src/env.rs`: the fallback removed; the error.
- `daemon/tests/config_selection.rs`: the red test (an unconfigured
  serve previously started and hung until the test timeout).

## Notes

A daemon cannot prompt; the error is the wayfinding answer.
