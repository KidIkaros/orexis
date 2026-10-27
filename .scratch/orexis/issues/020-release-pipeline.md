# 020 — Release pipeline

- **Type:** feature
- **Blocks:** M9
- **Status:** open
- **Claimed by:** —

## Question

The release pipeline (ADR-0019): a script + CI that builds the four
targets from a tagged commit, produces archives + sha256 + minisign
signatures + the `latest.json` manifest, and publishes the GitHub
Release. The minisign keypair must be generated, the public key pinned in
the repo, the private key held as a CI secret (ADR-0009).

## Evidence

- ADR-0019: artifacts, signing, channels, supply-chain floor.
- opencode's release shape (per-artifact checksums + manifest) is the
  pattern to match.
- The Tauri updater's keypair is the same minisign key (ADR-0019).

## Acceptance

- [ ] `scripts/release.sh` builds the four targets, archives, checksums,
      signs, and writes the manifest — dry-run mode first.
- [ ] CI publishes on a tag; the private key is a CI secret; local
      builds are never published.
- [ ] The public key is pinned in the repo and printed by the install
      script.
- [ ] The install script (curl | bash) verifies before it executes
      anything; `--no-modify-path` honored.
- [ ] A stranger install completes `orexis init` on Linux and macOS
      (manual, the M9 gate).

## Notes

The desktop bundles ride the same release; their updater artifacts come
from Tauri's build.
