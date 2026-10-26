# ADR-0002 — Provider catalog and profile

- **Status:** accepted
- **Date:** 2026-10-25
- **Deciders:** project owner, Devin
- **Ticket:** `.scratch/orexis/issues/002-provider-profile-schema.md`

## Context

Orexis is model-agnostic: inference runs wherever the user points it, remote or local. The provider set must be broad (Hermes ships 43 profiles) without a code change per provider, and the harness must work offline. Credentials must not leak into config or logs. The scheduler needs a per-provider concurrency capacity, and failures need a predictable fallback path.

## Decision

**Profile shape.** A rich typed core plus an `extra` map:

- identity: `id`, `display_name`, `kind`
- `base_url`
- `auth`: kind (`api_key`, `oauth`, `adc`, `sigv4`, `none`), env names, keyring ref, header format
- `wire`: dialect, tool format (`native`, `xml`, `json`), stream format (`sse`, `jsonl`), request transforms
- `cache`: dialect (`anthropic`, `openai`, `gemini`, `none`), breakpoint rules
- `thinking`: supported, parameter name, levels
- `capabilities`: tools, parallel tools, vision, JSON mode, reasoning
- `models`: list of `ModelInfo` (id, context, max output, pricing, capabilities)
- `limits`: `max_concurrency`, rate limits, retry policy
- `privacy`: data policy, retention, region
- `fallbacks`: ordered profile ids
- `extra`: free-form map for quirks that do not warrant a field

**Catalog.** A pinned snapshot generated from models.dev plus curated profiles, shipped in-tree. `orexis providers refresh` updates it. The harness works fully offline; a missing model is not an error.

**Precedence.** Shipped < user (`~/.config/orexis/providers/`) < project (`.orexis/providers/`) < env/CLI. `orexis providers show <id>` prints the winning profile and why.

**Secrets.** OS keyring by default (Secret Service, Keychain, Credential Manager); environment fallback; never plaintext config. Tokens are redacted from logs by the session writer (ADR-0001).

**Unknown models.** Conservative dialect defaults, a visible warning, and a one-line override.

**Capacity.** The profile declares `max_concurrency`; local defaults to 1, remote to 4; the user may override. The scheduler reads this value (invariant 17).

**Fallbacks.** Per-profile ordered chain; triggers on auth failure, 429 after retries, or repeated 5xx; a session may override.

**OAuth.** The adapter owns the device flow, stores tokens in the keyring, refreshes on 401, and never logs tokens.

## Consequences

- Adding a provider is data, not code, unless the wire genuinely differs.
- Offline operation and reproducible catalogs; refresh is explicit.
- Precedence is inspectable, so "why is it using that model?" has an answer.
- Keyring integration adds platform-specific work (Linux, macOS, Windows).
- Conservative defaults may surprise users; the warning and the override mitigate it.
- OAuth requires device-flow code per OAuth provider, but keeps tokens out of config.

## Alternatives considered

- **Live models.dev only** — always fresh, but breaks offline and reproducibility.
- **Minimal profile + per-provider code** — maximum fidelity, maximum maintenance; rejected for breadth.
- **Free-form YAML** — flexible, but unvalidatable and hard to migrate.
- **Env-only secrets** — simple, but leaks into shell history and process environments.
- **Global concurrency cap** — simple, but thrashes local models or underuses remote APIs.
- **Manual token paste** — no OAuth code, but poor UX and token sprawl.
