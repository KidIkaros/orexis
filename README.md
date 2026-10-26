# Orexis

A **sovereign, model-agnostic coding harness with a living entity**. Rust. TUI + desktop. Your state, identity, memory, sessions, and keys live on your machine; inference goes wherever you point it.

**Status:** pre-M1. Day 0 — planning and study scaffolding. No Rust code yet.

## What this repo is

| Path | Holds |
|---|---|
| `docs/orexis/PLAN.md` | The map: destination, decisions, frontier, fog, out of scope — plus the reference material behind it |
| `docs/orexis/study/` | The eight-repo study: Kindred, Harness, Familiar, DeepSeek Harness, opencode, Hermes, MUE-X, Mercury |
| `docs/orexis/adr/` | Architecture decision records |
| `.scratch/orexis/issues/` | Decision tickets |
| `CONTEXT.md` | The ubiquitous language |
| `AGENTS.md` | Rules for agents and contributors |

## Where to start

Read `docs/orexis/PLAN.md` **§Map** first. The **Frontier** is the next work: eleven open decisions, ordered, each with what it blocks. Do not begin M1 until the frontier's blocking decisions are resolved.

## What Orexis is built from

Orexis is assembled from two Rust codebases already owned by this project — **Kindred** (entity substrate, MCP/UTCP clients, orchestration) and **Harness** (tool loop, codec, providers, eval) — with architecture patterns from DeepSeek Harness, opencode, Hermes, MUE-X, and Mercury. The one-way porting ledger is in the plan (§6).

## Non-negotiables

- Dependencies point inward; ports are consumer-side; adapters are constructed only in the composition root.
- `tokio` is banned inward of adapters.
- Protocols are sans-io.
- No vector RAG over code.
- No telemetry.
- No wholesale plan rewrites: decisions append to the map index.

## License

MIT (expected; `LICENSE` lands with the first code commit).
