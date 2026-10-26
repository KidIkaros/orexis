# M1-08 — Import-lint, seam graph, and the M1 gate

- **Blocked by:** M1-01 … M1-07
- **Blocks:** M2
- **Status:** resolved (devin, 2026-10-26) — automated gate and provider checks green; direct Anthropic and a live-model repo task remain manual

## Goal

Enforce the Dependency Rule in CI, generate the seam graph, and run the full M1 acceptance gate.

## Acceptance

- [x] Import-lint fails on a deliberate `usecases -> adapters-*` import.
- [x] Import-lint fails on a deliberate `tokio` import in `core`, `ports`, or `usecases`.
- [x] The seam graph is generated and current (`docs/orexis/seams.md`).
- [x] `cargo fmt --check`, `clippy -D warnings`, and `cargo test --workspace` pass.
- [x] All three providers complete a scripted tool-call task: OpenAI-compatible (OpenRouter free + Refuseless), Anthropic schema (OpenRouter `/v1/messages` + free router), llama.cpp (local MiniCPM5-2B). *(Direct `api.anthropic.com` auth/version remains manual.)*
- [x] Replay is byte-identical over a recorded session.
- [x] `kill -9` recovery loses no acknowledged event (`restart.rs`).
- [x] The TUI renders all eleven states.
- [x] Attach → prompt → detach → reattach replays.
- [ ] One end-to-end run on a real repository task with a live model. *(The M3 test uses a scripted provider; the live probe covered a temp project, not a repo.)*
- [x] `docs/orexis/specs/M1.md` is updated to reflect what actually shipped.

## Notes

- `scripts/m1-gate.sh` runs the automated gate; `scripts/check-boundaries.sh` and `scripts/seam-graph.sh` are CI steps.
- The real composition root (`build_runtime`) wires catalog → provider → codec → context; the daemon owns the SQLite store.
- `serve` uses the real runtime by default and `OREXIS_FAKE=1` for tests.
- Live providers verified through OpenRouter free, Refuseless, and local llama.cpp; direct Anthropic is the only provider check still manual.
