# 010 — Packaging and signing

- **Type:** research
- **Blocks:** M9
- **Status:** resolved
- **Claimed by:** devin, 2026-10-27

## Question

How is Orexis packaged, signed, and upgraded on Linux and macOS — and how does the 1.0 checklist's signed release artifact work end to end?

## Evidence

- Plan §5.1: signed release artifact; `init` selects a provider; export/import; upgrade preserves sessions.
- Mercury: one-line installer, checksum, attach UX.
- Plan §11: packaging is an open study output.
- opencode (2026): curl script + brew + npm + checksums + `latest.json` manifest; desktop bundles per platform.
- dist (cargo-dist): automates builds/checksums/CI; artifact signing is Windows-only today.
- Tauri updater: minisign signatures mandatory, cannot be disabled — one keypair can serve CLI artifacts and desktop updates.

## Resolution

**One `orexis` binary, four targets, minisign-signed artifacts, GitHub
Releases primary, curl script + `orexis upgrade` as the user paths.**
Full decision in ADR-0019. The essentials:

- M9 consolidates the binaries into one `orexis` with subcommands; no npm
  wrapper (a second supply chain for no capability).
- Every artifact carries sha256 + a minisign signature; one minisign
  keypair serves the CLI artifacts and the Tauri updater; the public key
  is pinned in the repo; the private key is a CI secret / keyring entry
  (ADR-0009).
- The install script and `orexis upgrade` verify before they execute
  anything; no account, no telemetry; the release host is the only
  network contact.
- Upgrade preservation is structural: the binary is stateless; sessions
  and the entity live in `~/.config/orexis` and the project's `.orexis/`.
- Notarization and a brew tap are post-1.0; sigstore may be added as a
  secondary attestation.

## Follow-ups

- Ticket 019: `orexis` binary consolidation (prerequisite for packaging).
- Ticket 020: the release pipeline (script, CI, minisign, manifest).
- Ticket 021: `orexis upgrade` + the upgrade-preserves-state test.
