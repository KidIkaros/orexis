# 014 — `init` and `providers` surface implemented auth kinds

- **Type:** feature
- **Blocks:** —
- **Status:** resolved
- **Claimed by:** devin, 2026-10-27

## Question

How does `init` (and `providers show`/`list`) make it visible *before*
selection that a profile's auth kind is not implemented (OAuth device flow,
SigV4, ADC), so the user never picks a profile that cannot build?

## Evidence

- ADR-0016: auth triggers adapter work on demand; until then such profiles
  fail at composition.
- 1.0 checklist: "`orexis init` selects a provider; there is no silent
  default." A profile that fails at first turn is a silent default in
  disguise.
- ADR-0002: precedence is inspectable; "why is it using that model?" has an
  answer — the same inspectability applies to auth support.

## Resolution

**Buildability is visible at every layer, and refused at the earliest
one:**

- `composition::auth_kind_implemented(kind)` — composition is the honest
  home: it is the only crate that knows what it constructs; the catalog
  stays pure data. `ApiKey` and `None` build; `OAuth`, `Adc`, `SigV4`,
  `ExternalProcess` do not.
- `build_runtime` refuses an unimplemented auth kind with an error naming
  it (ADR-0016's rule made real), and the unimplemented wires
  (`bedrock_converse`, `codex_responses`) get the same naming treatment
  as Gemini's.
- The hermes loader **no longer drops** the nine unimplemented-auth
  profiles (4 oauth_external, oauth_device_code, copilot, aws_sdk,
  vertex, external_process) — they load as data and are *marked*, not
  invisible. `AuthKind::ExternalProcess` and
  `ProviderKind::{Bedrock, CodexResponses}` were added for them.
- `orexis providers list` marks each profile's auth kind and credential
  presence; `providers show <id>` says "cannot build; see ADR-0016" for
  unimplemented kinds.
- `orexis init` refuses an unimplemented auth kind or unknown profile and
  writes nothing; warns on an unknown model (conservative defaults apply)
  and on a missing credential; interactive picker shows unimplemented
  profiles but they are not selectable.
- The selection `init` writes (`.orexis/config.yaml`) is what `serve`
  builds from when the environment is silent; the environment overrides
  it — proven by spawn tests (a broken selection fails startup naming
  the profile; an env override wins).

## Landed

- `composition`: `auth_kind_implemented`, `read_selection`/
  `write_selection`/`Selection`, `load_catalog`, the auth refusal, the
  Bedrock/CodexResponses arms; `serde_yaml` dep.
- `adapters-provider-catalog`: `AuthKind::ExternalProcess`,
  `ProviderKind::{Bedrock, CodexResponses}`; the hermes loader maps all
  auth types and wire modes (unknown ones warn and skip); the structural
  test allows the `acp://` scheme.
- `orexis` binary: `providers list/show`, `init` (interactive +
  `--profile/--model`), the refusal and warning paths, tests.
- `daemon`: env reads the written selection (env > config > default);
  spawn-based tests.

## Follow-ups

- The silent default (env > config > "openai"/"gpt-5") still stands; the
  1.0 checklist's "no silent default" removal — real-mode serve errors
  "run orexis init" when nothing is configured — is its own ticket with
  its own red test.
