# AGENTS.md — rules for anyone (human or agent) working in Orexis

Read `docs/orexis/PLAN.md` §Map and `CONTEXT.md` before changing anything.

## Status

Pre-M1. This repo currently holds documentation only. The Rust workspace arrives with M1, after the frontier's blocking decisions are resolved.

## Architecture (enforced, not advisory)

1. **The Dependency Rule.** Dependencies point inward:
   `orexis-core` ← `orexis-ports` ← `orexis-usecases` ← `orexis-adapters-*` ← `orexis-composition` ← `orexis` (binary).
2. **Ports are consumer-side.** A port trait lives in `orexis-ports`, never in an adapter crate.
3. **Adapters are constructed only in `orexis-composition`.** No other crate may `new` a concrete adapter.
4. **No adapter type crosses a port.** Ports exchange plain data structures only.
5. **`tokio` is banned inward of adapters.** `core`, `ports`, and `usecases` may not name it. Time, spawn, and cancellation are ports.
6. **Protocols are sans-io.** Byte framing, SSE parsing, tool-call parsing, and protocol encoding are sync and runtime-free.
7. **No shared `utils` crate.** It is the zone-of-pain magnet.
8. **CI enforces 1–7 with import-lint.** A violation fails the build.

## Async

- Ports may be `async fn`; use `async-trait` for dyn-friendly ports.
- Streaming crosses ports as `futures::Stream`.
- `ToolExecutor` is async; the sandbox adapter owns `spawn_blocking`. No `block_in_place` in the loop.
- Tests use fake `Clock`/`Spawner` and `futures::executor::block_on`; `#[tokio::test]` belongs in adapter tests only.

## Code

- Idiomatic Rust; `rustfmt` and `clippy -D warnings` are gates.
- `thiserror` in libraries; `anyhow` only in binaries.
- No `unwrap`/`expect` in library code on paths that can fail from input or I/O.
- Comments explain **why**, never **what**. Do not add comments to code you did not change.
- Prefer the smallest correct change. Do not refactor adjacent code inside a feature ticket.

## Testing and review

- Every ticket starts with a failing test that encodes its acceptance check.
- Two-axis review before commit: **Standards** (this file, `CONTEXT.md`, idioms) and **Spec** (does it do what the ticket said).
- `verification-loop` before a ticket closes.
- `santa-method` (two independent adversarial reviewers) on the session log, scheduler, evolution, and permissions.
- E2E tests use a temporary home directory; never write to a real user home.

## Process

- **One decision ticket per session.** Claim it before working. Resolve it, append one line to the map's *Decisions so far*, and stop.
- **No wholesale plan rewrites.** Decisions append; the body changes only when the frontier or fog changes.
- **Reversing a settled decision requires an ADR** with the new evidence. Not a rewrite.
- **No silent refactors.** Findings become tickets.
- **Bugs:** no fix without a command that goes red on *that* bug, then a regression test.

## Security

- No code reuse from repositories without a `LICENSE` file.
- Untrusted content (tool output, web results, issue text, remote agents) is delimited and never treated as instructions.
- Secrets come from the environment or the OS keyring; never commit them, never log them.
- No telemetry. `/self` must show exactly what leaves the machine.
- Third-party skills are packages: trust tiers, scanning, quarantine.

## Anti-patterns (do not reintroduce)

- Agentic runtime frameworks (LangChain, LangGraph, AutoGen, CrewAI, LlamaIndex, Pydantic AI, ADK, Semantic Kernel).
- Vector RAG over source code.
- Line-number-based edits.
- Unbounded tool output.
- Confirmation dialogs where undo would do.
- Harness identity mimicry (presenting as another product to use its backend).

## Commands

Once the workspace exists:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Until then, documentation changes only.

## Where things live

| Kind | Path |
|---|---|
| Map, decisions, frontier, fog | `docs/orexis/PLAN.md` §Map |
| Decision tickets | `.scratch/orexis/issues/` |
| Architecture decisions | `docs/orexis/adr/` |
| Per-repo study | `docs/orexis/study/` |
| Language | `CONTEXT.md` |
