# M1-17 — Composition correctness

- **Blocked by:** —
- **Blocks:** M2, M3
- **Status:** resolved (devin, 2026-10-26)

## Goal

Fix three real-runtime gaps: XML codec selection, provider capacity, and the shipped catalog path. Surface the unknown-model warning.

## Acceptance

- [x] A profile with `tool_format: xml` selects `xml-manual`.
- [x] `Runtime.generation_slots` comes from `profile.limits.max_concurrency`; `serve` uses it.
- [x] The unknown-model warning reaches the operator (`Runtime.warnings`, printed by `serve`).
- [x] The providers directory is found at runtime: `OREXIS_PROVIDERS`, exe-relative `../share/orexis/providers`, then the dev fallback.
- [x] Three composition integration tests cover the codec matrix, capacity, and the warning.

## Notes

- `BuildConfig.providers_dir` lets tests inject a catalog without mutating process env.
- The dev fallback uses `CARGO_MANIFEST_DIR`; a release install ships `share/orexis/providers`.
