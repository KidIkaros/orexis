# M2-06 — Settings, privacy, evaluation, and gate

- **Blocked by:** M2-02 … M2-05
- **Blocks:** —
- **Status:** in progress (devin, 2026-10-26) — settings/accessibility/gate done; evaluation cycles blocked; Tauri shell unbuilt

## Goal

Settings and privacy, accessibility, the two evaluation cycles, and the M2 gate.

## Notes

- Settings are read-only: profile/model come from the daemon config; changing them needs a restart.
- Privacy lists the provider/model, data sent, no telemetry/HTTP, keyring/env keys, and local session/checkpoint storage.
- The gate checks the UI wasm build, no HTTP server, and the six panel components, then runs the M3 automated gate.
- Evaluation and usability are blocked, not passed.

## Acceptance

- [x] Settings modal: profile/model, plan mode, permission summary, privacy list. *(Read-only; editing requires restart.)*
- [x] Accessibility: focus-visible rings, ARIA labels/roles, aria-live, reduced motion, prefers-contrast, status text beside color.
- [ ] Evaluation cycles recorded. *(Protocol and decision-log template are in `desktop-evaluation.md`; not run — no participants, Tauri shell unbuilt.)*
- [ ] Usability rows measured. *(Not measured; no evaluation cycle has run.)*
- [x] `scripts/m2-gate.sh` green (automated; manual items printed).
