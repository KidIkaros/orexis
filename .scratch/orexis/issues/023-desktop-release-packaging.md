# 023 — Desktop workspace release packaging (v0.2.0, M2)

- **Type:** task
- **Blocks:** v0.2.0
- **Status:** resolved
- **Claimed by:** ikaaros + antigravity, 2026-10-29

## Question

How should the Orexis desktop application (`crates/desktop` Tauri v2 + Leptos frontend) be packaged, signed, and integrated with the v0.1.0 release pipeline for multi-platform distribution on Linux and macOS?

## Evidence

- `crates/desktop/src-tauri/tauri.conf.json`: Tauri v2 configuration.
- `crates/desktop/ui`: Leptos WebAssembly frontend with six oversight panels (chat, fleet, approvals, diffs, artifacts, files).
- `crates/desktop-core`: Pure Rust bridge and diff model crate connecting to the daemon socket.
- ADR-0008: Desktop three-zone oversight workspace.
- ADR-0010: Shared outer layer for desktop and client crates.
- ADR-0019: Minisign signing key shared across CLI and desktop artifacts.
- `scripts/m2-gate.sh`: Automated desktop test gate.

## Resolution

1. **Hunk-aligned side-by-side split diff & automatic expansion (`orexis-desktop-core::diff` + `crates/desktop/ui/src/diff.rs`)**:
   - Implemented `align_split_diff` and `classify_diff_line` in `crates/desktop-core/src/diff.rs` so deletions (`-`) and additions (`+`) within each modification chunk align row-by-row with `.pad` cells for unbalanced blocks, keeping left and right panes strictly synchronized.
   - Added `AppState::diff_expanded` (`should_auto_expand_diff` threshold > 40 lines) so large diffs automatically open full-screen when `ServerMsg::Diff` arrives (`M2-03`).
2. **Tauri v2 packaging and signed release pipeline (`scripts/release.sh`, `scripts/install.sh`, `.github/workflows/release.yml`)**:
   - Added `scripts/release.sh desktop TARGET [OUTDIR]` building the Leptos `wasm32-unknown-unknown` frontend via `trunk build --release` and compiling `orexis-desktop` (`crates/desktop/src-tauri`), archiving `orexis-desktop-<version>-<target>.tar.gz`, computing `.sha256`, and signing with `minisign`.
   - Extended `build_manifest` to emit `"desktop_artifacts"` in `latest.json` alongside `"artifacts"`, preserving backwards compatibility with `orexis upgrade`.
   - Added `--desktop` flag to `scripts/install.sh` (`curl ... | bash -s -- --desktop`) verifying manifest signature, SHA-256 checksum, and artifact signature before installing `orexis-desktop`.
   - Added the `desktop` matrix job (`x86_64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`) to `.github/workflows/release.yml`.

## Follow-ups

- Tag `v0.2.0` when ready to publish the first combined CLI + Desktop release on GitHub.
- Record human participant sessions in `docs/orexis/prototypes/desktop-evaluation.md` once the packaged desktop binary is exercised by external testers.
