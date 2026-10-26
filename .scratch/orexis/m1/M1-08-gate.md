# M1-08 — Import-lint, seam graph, and the M1 gate

- **Blocked by:** M1-01 … M1-07
- **Blocks:** M2
- **Status:** open

## Goal

Enforce the Dependency Rule in CI, generate the seam graph, and run the full M1 acceptance gate.

## Acceptance

- [ ] Import-lint fails on a deliberate `usecases -> adapters-*` import.
- [ ] Import-lint fails on a deliberate `tokio` import in `core`, `ports`, or `usecases`.
- [ ] The seam graph is generated and current.
- [ ] `cargo fmt --check`, `clippy -D warnings`, and `cargo test --workspace` pass.
- [ ] All three providers complete a scripted tool-call task.
- [ ] Replay is byte-identical over a recorded session.
- [ ] `kill -9` recovery loses no acknowledged event.
- [ ] The TUI renders all eleven states.
- [ ] Attach → prompt → detach → reattach replays.
- [ ] One end-to-end run on a real repository task.
- [ ] `docs/orexis/specs/M1.md` is updated to reflect what actually shipped.

## Notes

- The gate is a script (`scripts/m1-gate.sh`) so it can run locally and in CI.
- Any new scope discovered during M1 becomes a ticket, not an addition to this one.
