# Harness — study

**Source:** `/home/ikaaros/Coding/Gold/harness` · **Language:** Rust · **Size:** 21,665 LOC (incl. tests) · **License:** MIT · **Last seen:** local checkout

**Why it is in the study:** it is the tool loop — codec, providers, sandbox, genes, probe/compare eval, sessions, serve/attach — and the source of the probe discipline.

## Known surface

| Area | Modules |
|---|---|
| Core | `crates/core`: `types.rs` (Message, Action, ToolSpec, Observation, Transcript, SamplingParams, Budget), `affect.rs`, `assemble.rs`, `compact.rs` |
| Protocol | `crates/protocol`: `arm.rs` (`xml-manual`, `xml-native`, `json`, `lenient`), `xml.rs`, `json_schema.rs`, `lenient.rs`, `step.rs` |
| Providers | `crates/providers`: `llama_server.rs` (subprocess + OpenAI HTTP/SSE), `embedded.rs` (llama-cpp-4), `wire.rs`, `monitor.rs`, `advisor.rs`, `planner.rs`, `instance.rs` |
| Tools | `crates/tools`: `sandbox.rs`, `gate.rs`, `exec.rs` (bounded shell, process group, 64 KB caps) |
| Genes | `crates/genes`: fragments, `Drives`, ledger, seal |
| Eval | `crates/eval`: task YAMLs, deterministic judges, `probe_runner.rs`, `compare.rs` |
| Agent | `crates/agent`: `loop_impl.rs` (the turn loop), `events.rs` |
| CLI | `crates/cli`: `app.rs`, `repl.rs`, `session.rs` (SQLite), `serve.rs`/`attach.rs`/`proto.rs` (NDJSON over Unix socket), `models.rs`, `hw.rs`, `recipes.rs` |

## Dimensions to fill

1. **Process model** — REPL, daemon (`serve`), and attach; one generation at a time via a semaphore.
2. **Turn loop** — `loop_impl.rs`; how effects are interpreted; stall and budget handling.
3. **Session model** — SQLite schema; event sequence; resume; no fork.
4. **Tool protocol** — four codec arms; how multi-call XML is parsed; JSON-schema arm.
5. **Context management** — `assemble.rs` priority sections; `compact.rs`; what is logged.
6. **Provider abstraction** — llama-server vs embedded vs remote; wire profiles; advisor/planner.
7. **Entity substrate** — `Affect` (frustration/confidence) and genes drives; no identity.
8. **Evolution** — genes ledger/seal/suggest; no mutation engine.
9. **Interop** — none (no ACP/MCP/A2A/UTCP).
10. **Surfaces** — REPL only; no TUI, no desktop.
11. **Quality practice** — probe/compare, deterministic judges, 112 tests, clippy-clean.
12. **Failure modes** — single-model assumptions; sync `ToolExecutor` + `block_in_place`; no protocols.

## Known evidence

- `Provider` is `#[async_trait]`; `ToolExecutor` is **sync** and called inside `tokio::task::block_in_place` — a runtime leak we are rejecting.
- `serve`/`attach` is NDJSON over a Unix socket; `proto.rs` explicitly notes the transport is a detail.
- Four codec arms are probe-verified; `xml-manual` is the production arm for the local model.
- Probe/compare is a paired, deterministic eval loop — the seed for `orexis-bench`.
- Genes have a ledger and a seal but no mutation loop.

## Adopt

- <to fill>

## Adapt

- <to fill>

## Reject

- <to fill>

## Open questions

- How does `step.rs` decide between `CallTool`, `Say`, `Done`, and `Reprompt`?
- How are multi-call XML blocks ordered and executed?
- What exactly does the seal hash, and when does it fail?
- How does `serve` serialize generations, and what does `attach` replay?
