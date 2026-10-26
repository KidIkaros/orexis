# ADR-0009 — Secrets and crypto boundary

- **Status:** accepted
- **Date:** 2026-10-26
- **Deciders:** project owner, Devin
- **Related:** ADR-0002 (provider catalog and credentials)

## Context

Provider credentials are API keys and OAuth tokens. ADR-0002 decided the OS
keyring by default, environment fallback, never plaintext config. The project
owner also maintains `origin-crypto-sdk` and `origin-tools`; the question was
whether to use them for the keyring.

Evidence:

- `origin-crypto-sdk` is Apache-2.0 but marked **experimental, not for
  production use**, with no independent audit. Its custom NTRU Prime, Ed41417,
  native ChaCha20/Poly1305, hybrid signing, and seed-blob format are unreviewed;
  on-disk formats may change between patch releases.
- `origin-tools` stores a **file-based encrypted master seed**
  (`~/.origin/identity.seed`, Argon2id + XChaCha20-Poly1305), passphrase-derived.
  It has no OS-keyring integration.
- Both licenses permit reuse; the blocker is security maturity, not licensing.

## Decision

1. **Provider credentials live in the OS keyring**, with environment fallback.
   The keyring stores opaque tokens only; no plaintext config, no logging.
2. **No experimental crypto for production secrets.** Unreviewed custom crypto
   is not a keyring.
3. **`origin-crypto-sdk` and `origin-tools` are references, not dependencies
   for the keyring.** Their `IdentityStore`/`Envelope`/passphrase patterns are
   useful design input.
4. **Future, non-keyring uses:**
   - Entity identity and evolution signing (M5/M6) may evaluate hybrid
     signatures **after an independent audit**.
   - Opt-in encrypted exports/backups use vetted primitives
     (`XChaCha20-Poly1305`, `Argon2id`) directly, or the SDK only behind a
     feature flag with a visible warning. Never the default.
5. **Headless fallback:** environment variables. A passphrase-protected file
   vault is a later option built on vetted primitives, not the SDK's custom
   blob format.

## Consequences

- The keyring remains small and platform-integrated.
- The entity/evolution work can still use the owner's crypto work where it
  earns its place, with the audit gate explicit.
- No format lock-in from an experimental blob.
- The keyring adapter is a thin `SecretStore` implementation; the port stays
  consumer-side.

## Alternatives considered

- **Use the SDK's seed blob as the keyring** — rejected: unaudited custom
  crypto, passphrase-at-start problem, format churn.
- **Use `origin-tools` `origin-pass` as the vault** — rejected for provider
  credentials: file vault, not an OS keyring; useful as a reference.
- **Vendor the SDK now for future signing** — rejected: no current need; the
  audit gate would be bypassed.
