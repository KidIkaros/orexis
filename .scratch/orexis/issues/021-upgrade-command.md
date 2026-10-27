# 021 — `orexis upgrade`

- **Type:** feature
- **Blocks:** M9
- **Status:** resolved
- **Claimed by:** devin, 2026-10-27

## Question

`orexis upgrade` (ADR-0019): fetch the release manifest, verify the
signature against the pinned key, replace the binary, preserve all state.
The 1.0 checklist's "upgrade preserves sessions and the entity" is proven
by a test, not a promise.

## Evidence

- ADR-0019: upgrade is structural — the binary is stateless; sessions and
  the entity live in `~/.config/orexis` and the project's `.orexis/`.
- 1.0 checklist: upgrade preserves sessions and the entity.
- The manifest (`latest.json`) is the upgrade source of truth.

## Resolution

**The upgrade verifies in Rust** — the user's machine may not have the
minisign CLI, so `crates/orexis/src/minisign.rs` implements verify-only
signature checking on `ed25519-dalek` + BLAKE2b-512 (the hashed format
the pipeline produces; legacy `Ed` signatures are refused). Interop is
proven against committed fixtures produced by rsign, plus five negative
cases (tampered file, tampered trusted comment, another key, legacy tag,
truncation).

The flow: manifest + signature → verify against the pinned key
(`crates/orexis/orexis.pub`, a placeholder until the wizard step pins the
real key — a clear error, not a cryptic one) → the host target's artifact
→ sha256 against the manifest → extract (system `tar`; the installer
requires it too) → atomic replace (temp file in the binary's directory,
chmod 0755, rename over `current_exe` — the running process keeps the
old inode). Version compare is numeric with zero-padding (`0.1` =
`0.1.0`); an equal or older manifest is "already at the newest version",
so downgrades are refused here — reinstalling an older artifact via the
installer's `OREXIS_VERSION` is the documented downgrade.

`--from DIR` sources the manifest and artifact locally (offline installs
and the tests), the same hook `install.sh` has.

## Landed

- `crates/orexis/src/minisign.rs`: verify-only, hashed format, fixtures
  (a committed TEST keypair's public side + rsign-produced signatures;
  no secret material is committed — the test builds keypairs on dalek).
- `crates/orexis/src/upgrade.rs`: the flow, version compare, checksum,
  extraction, atomic replace; `--from`; the state-preservation test
  through the full verified path (signature → checksum → replace →
  state byte-identical).
- `orexis --version` (the upgrade's comparison needs it).
- Deps: `ed25519-dalek`, `blake2`, `sha2`, `base64`,
  `orexis-adapters-http` (the download transport).

## Follow-ups

- The release CI bakes `OREXIS_RELEASE_REPO` into the binary at build
  time (a build.rs reading the env) so a released binary upgrades
  without env setup — polish for the first real release.
