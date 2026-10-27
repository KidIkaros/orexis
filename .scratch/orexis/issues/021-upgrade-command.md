# 021 — `orexis upgrade`

- **Type:** feature
- **Blocks:** M9
- **Status:** open
- **Claimed by:** —

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

## Acceptance

- [ ] `orexis upgrade` verifies before it replaces anything; a failed
      verification leaves the installed binary untouched.
- [ ] The upgrade test: state created under vN (sessions, entity,
      corridor) survives an upgrade to vN+1.
- [ ] A downgrade path is documented (reinstall an older artifact) or
      explicitly rejected with a reason.

## Notes

E2E tests use a temporary home directory; never write to a real user
home (AGENTS.md).
