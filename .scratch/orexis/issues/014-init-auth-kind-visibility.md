# 014 — `init` and `providers` surface implemented auth kinds

- **Type:** feature
- **Blocks:** —
- **Status:** open
- **Claimed by:** —

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

<filled when resolved>

## Follow-ups

<new tickets or fog this resolution creates>
