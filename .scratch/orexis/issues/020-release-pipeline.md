# 020 — Release pipeline

- **Type:** feature
- **Blocks:** M9
- **Status:** resolved (automated half; key generation is a user step)
- **Claimed by:** devin, 2026-10-27

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

## Resolution

**`scripts/release.sh` (archive / manifest / dry-run modes) +
`scripts/install.sh` (curl | bash, `--from DIR` for offline and tests) +
`.github/workflows/release.yml` (tag-triggered, matrix, signed publish).**

- `release.sh archive TARGET [OUTDIR]` — release build, tar.gz with the
  binary + docs, sha256, minisign signature. Honors `CARGO=cross` for
  cross-compilation; signs with `minisign` or `rsign` (same format,
  different CLIs).
- `release.sh manifest ARTDIR OUTDIR` — assembles `latest.json` (version,
  per-target url + sha256) from the artifacts, refusing unsigned ones,
  then signs the manifest. The manifest's signature is what makes the
  checksums trustworthy.
- `release.sh dry-run` — archives the host target without signing; the
  pipeline's build half is exercised without a key.
- `install.sh` — detects os/arch, fetches the manifest (+ its
  signature), verifies the manifest signature when the key is pinned
  (a loud warning while `MINISIGN_PUB` is empty), downloads the target's
  artifact, verifies sha256 against the manifest, verifies the artifact
  signature when possible, installs to `~/.local/bin`, prints the
  `orexis init` next step. `--from DIR` sources everything locally —
  offline installs and this script's own tests.
- CI: a tag push runs the four-target matrix (cross for aarch64-linux),
  uploads per-target artifacts; the publish job flattens, builds and
  signs the manifest, and creates the release. The private key comes
  from the `OREXIS_SIGNING_KEY` secret.

**Verified here** (throwaway keypair, fixture artifacts): manifest
assembly + signature verification; installer end-to-end from a local dir
(checksum + signatures verified, binary installed and runs); tampered
artifact refused (checksum mismatch, nothing installed); tampered
manifest refused (signature failure, nothing installed); dry-run release
build.

## The user's steps (wizard — only you can do these)

1. Generate the production keypair:
   `minisign -G -p orexis.pub -s orexis.key` (or `rsign generate -p
   orexis.pub -s orexis.key -W`). Keep `orexis.key` in your OS keyring or
   a password manager; it never enters the repo.
2. Pin the public key: paste the two lines of `orexis.pub` into
   `MINISIGN_PUB` in `scripts/install.sh`.
3. Add the private key as the GitHub secret `OREXIS_SIGNING_KEY`
   (`gh secret set OREXIS_SIGNING_KEY < orexis.key`).
4. Add a git remote and push; tag `v0.1.0` to exercise the pipeline for
   real; then bake `OREXIS_RELEASE_REPO`'s default into
   `scripts/install.sh`.
5. The same keypair signs the Tauri desktop updates (ADR-0019) — set it
   as `TAURI_SIGNING_PRIVATE_KEY` when the desktop ships.

## Acceptance

- [x] `scripts/release.sh` builds, archives, checksums, signs, writes
      the manifest — dry-run mode first.
- [x] CI publishes on a tag; the private key is a CI secret; local
      builds are never published.
- [ ] The public key is pinned in the repo and printed by the install
      script. (user step 2)
- [x] The install script verifies before it executes anything.
- [ ] A stranger install completes `orexis init` on Linux and macOS
      (manual, the M9 gate).

## Notes

The desktop bundles ride the same release; their updater artifacts come
from Tauri's build.
