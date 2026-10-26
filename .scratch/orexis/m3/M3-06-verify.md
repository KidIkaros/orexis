# M3-06 — Verify-on-stop

- **Blocked by:** M3-03, M3-05
- **Blocks:** M3-08
- **Status:** resolved (devin, 2026-10-26)

## Goal

Refuse a `Done` claim until the configured test/build command passes, unless the user explicitly skips.

## Acceptance

- [x] A verify command is configured per project (`BuildConfig.verify_command`, `OREXIS_VERIFY`).
- [x] `Done` runs verify and is refused on failure, with the output attached as an `Error`.
- [x] The model can retry; an explicit `skip_verify` allows `Done` without running it.
- [x] A passing verify is recorded as `SessionEvent::Verify`.
- [x] Tests: failing verify blocks then passes on retry; skip allows.

## Notes

- `Verifier` is an async port; `ShellVerifier` reuses the sandboxed `run_shell` (timeout, caps, env scrub).
- Failure appends `Verify(false)`, `Error(output)`, and a `Reprompt`; the loop continues so the model can fix.
- The skip flag travels on `ClientMsg::Prompt { skip_verify }` and `TurnDeps.skip_verify`.
