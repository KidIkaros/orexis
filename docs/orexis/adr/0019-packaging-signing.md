# ADR-0019 — Packaging, signing, and upgrade

- **Status:** accepted
- **Date:** 2026-10-27
- **Deciders:** project owner, Devin
- **Related:** PLAN §5.1 (1.0 checklist), §11 (supply-chain risk), ADR-0009 (secrets), ADR-0016, issue 010, study: Mercury (installer lesson)

## Context

The 1.0 checklist demands: installs from a **signed release artifact** on
Linux and macOS; upgrade preserves sessions and the entity; no account, no
telemetry. Packaging is the last open frontier item (010) and blocks M9.

Evidence:

- **Mercury** (study): the one-line installer + checksum + attach UX are
  the adoption lesson — "distribution beats architecture".
- **opencode** (2026): curl install script (GitHub Releases tarballs,
  sha256), brew tap + official formula, npm, pacman, mise, nix; desktop
  via dmg/exe/deb/rpm/AppImage; per-artifact checksums plus a
  `latest.json`-style manifest.
- **dist (cargo-dist)**: automates tarballs, installers, checksums, and
  release CI — but its artifact signing is Windows-only today
  (macOS/Linux/sigstore are "🔜"). Its shell installers manage receipts
  and PATH.
- **Tauri** (the M2 desktop): the updater **requires** a minisign
  signature — it cannot be disabled; `tauri signer generate` produces a
  minisign keypair, the public key is embedded in `tauri.conf.json`.
- **Threat record** (§0): slopsquatting, TrustIssues (CVSS 10),
  Clinejection — supply-chain safety is a product surface, so the install
  path must verify before it executes anything.

## Decision

**One `orexis` binary, four targets, minisign-signed artifacts, GitHub
Releases as the primary channel, a curl install script and `orexis
upgrade` as the user paths.**

### Artifacts

- The M9 binary consolidation ships one `orexis` binary with subcommands
  (`serve`, `attach`, `acp`, `tui`, `bench`, `init`, `export`, `import`,
  `upgrade`, `providers`). No runtime dependency on Node/Python — npm
  wrappers are rejected (a second supply chain for no capability).
- Targets: `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`,
  `x86_64-apple-darwin`, `aarch64-apple-darwin`. musl variants are added
  when a user needs them, not speculatively.
- Per target: a `tar.gz` archive containing the binary, `README`,
  `LICENSE`, `NOTICE`, and a `sha256` checksum file — plus a minisign
  signature over the archive.
- The desktop (Tauri) ships its own bundles (dmg/deb/rpm/AppImage) with
  the updater artifacts Tauri generates.

### Signing

- **minisign (ed25519) over every release artifact**, with sha256
  checksums alongside. One keypair serves the CLI artifacts and the Tauri
  updater (Tauri's signer is minisign); the public key is pinned in the
  repository and printed by the install script.
- The private key is a production secret (ADR-0009): a GitHub Actions
  secret in CI, the OS keyring locally. It never enters the repo or logs.
- macOS Gatekeeper: the curl path and brew do not set the quarantine
  attribute, so they install without notarization. Browser downloads hit
  Gatekeeper until an Apple Developer ID exists — notarization is a
  follow-up when there is an identity to notarize with, not a 1.0 block.

### Channels and upgrade

- **GitHub Releases** is the primary channel; the release manifest
  (`latest.json`: version, artifact URLs, sha256s, minisign signature)
  is the upgrade source of truth.
- **Install script** (curl | bash, opencode pattern): detect os/arch →
  download archive + checksum + signature → verify sha256, then minisign
  against the pinned key → install to `~/.local/bin` → print `orexis
  init` next steps. `--no-modify-path` honored. The script verifies
  **before** it executes anything.
- **`orexis upgrade`**: fetch the manifest, verify signatures, replace
  the binary. State preservation is structural, not a migration: sessions
  and the entity live in `~/.config/orexis` and the project's `.orexis/`;
  an upgrade replaces the binary and touches no state. The checklist item
  is proven by an upgrade test (state created under vN survives vN+1).
- A Homebrew tap is a post-1.0 convenience channel; the formula carries
  the checksums, so brew users get verification from brew itself.

### Supply-chain rules (the floor)

- Verify before execute — the install script and `orexis upgrade` check
  signatures before running or replacing anything.
- No account, no telemetry anywhere in the install or upgrade path; the
  only network contact is the release host.
- Release builds come from CI on a tagged commit; local builds are never
  published.

## Consequences

- The 1.0 checklist's "signed release artifact" is concrete: minisign +
  sha256, key pinned, verify-before-execute.
- Upgrade safety is structural; no state migration lives in the
  packaging layer.
- One signing mechanism across CLI and desktop updater; one key to
  protect.
- The release pipeline is a small, auditable script — no dist
  dependency; dist becomes interesting only if the target matrix or
  installer variety grows.

## Alternatives considered

- **dist (cargo-dist) end to end** — rejected for 1.0: its signing does
  not cover Linux/macOS artifacts, so we would still own the minisign
  step; the remaining value (installer variety) is not needed yet.
- **sigstore keyless** — rejected as the primary: verification needs
  Rekor (network) and an OIDC identity; minisign is offline-verifiable
  and account-free, which is the sovereign position. Sigstore can be
  added as a secondary attestation later.
- **npm distribution** — rejected: a second supply chain (registry,
  runtime) for a self-contained Rust binary; the corpus's slopsquatting
  lesson applies directly.
- **Notarize now** — deferred: needs an Apple Developer identity; the
  curl/brew paths do not require it.

## Follow-ups

- Ticket 019: `orexis` binary consolidation (subcommands; prerequisite).
- Ticket 020: the release pipeline (script, CI, minisign, manifest).
- Ticket 021: `orexis upgrade` + the upgrade-preserves-state test.
- Ticket 014 (init auth-kind visibility) lands with the consolidated
  binary's `init`/`providers` subcommands.
