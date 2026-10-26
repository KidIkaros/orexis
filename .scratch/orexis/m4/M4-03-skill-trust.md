# M4-03 — Skill trust tiers and quarantine

- **Blocked by:** —
- **Blocks:** M4-04
- **Status:** resolved (devin, 2026-10-26)

## Goal

A skill package is loaded only when its trust tier and hash allow it; a tampered
or untrusted skill is quarantined.

## Acceptance

- [x] Manifest: name, version, tier, sha256, permissions (`crates/adapters-skills`).
- [x] SHA-256 of `SKILL.md` is verified; mismatch or missing hash → quarantined.
- [x] `community` is blocked by default; `quarantined` never loads.
- [x] `Skills::list` reports status; `load` returns text only for allowed tiers.
- [x] Tests: verified loads, tampered quarantined, community blocked/opt-in, quarantined tier, missing hash.

## Notes

- No skill execution in M4; this is the supply-chain foundation.
- A new `adapters-skills` crate keeps it out of core.
