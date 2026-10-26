# M4-03 — Skill trust tiers and quarantine

- **Blocked by:** —
- **Blocks:** M4-04
- **Status:** open

## Goal

A skill package is loaded only when its trust tier and hash allow it; a tampered
or untrusted skill is quarantined.

## Acceptance

- [ ] Manifest: name, version, tier (`trusted`/`verified`/`community`/`quarantined`), sha256, permissions.
- [ ] Hash of `SKILL.md` is verified; mismatch → quarantined.
- [ ] `community` is not loaded by default; `quarantined` never loads.
- [ ] Registry lists status; load returns the text only for allowed tiers.
- [ ] Tests: valid verified, tampered quarantined, community blocked.

## Notes

- No skill execution in M4; this is the supply-chain foundation.
- A new `adapters-skills` crate keeps it out of core.
