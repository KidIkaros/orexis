# ADR-0021 — Specialized harness classes per strategy (OO-Harness pattern)

- **Status:** proposed
- **Date:** 2026-10-28
- **Deciders:** Ikaaros

## Context

Orexis ships one global harness whose `Strategy` enum (`plan`, `build`, `review`, `mine`) selects a use case but shares one execution strategy, one context budget, and one tool set. The OO-Harness paper (Li et al., arXiv 2610.28380) shows that a single global harness struggles with heterogeneous tasks: component ablations find that adding memory, tools, and specialist agents improves hard tasks while *degrading* easy ones, and that evolving a hierarchy of specialized harness subclasses — each inheriting shared code and overriding execution strategy — with a router selecting per task, achieves up to +15.79% over the base harness on long-horizon benchmarks.

Orexis already has the substrate: strategies are use cases (ADR-0013/0020), workers share identity, and the scheduler is capacity-bounded. The missing piece is making each strategy a *specialized harness class* with its own context budget, tool subset, and compaction policy, selected by a router informed by execution records.

## Decision

Adopt the OO-Harness pattern as the M9+ optimization layer, without changing the M1–M8 architecture. The specialization is expressed as **data + a pure pipeline**, not class inheritance (per functional-design), composed behind a port (per clean-architecture):

1. **A harness class is a `HarnessConfig` value + a pure `run` pipeline.** Each strategy (`plan`, `build`, `review`, `mine`) is a config struct (context budget, tool subset, compaction threshold, verification posture, prompt prefix) plus a pure `run(config, input) -> output` pipeline. `BaseHarness` is `HarnessConfig::default()` — the lean minimal tool set and tightest context budget.
2. **Composition over inheritance.** `BaseHarness` is the default implementation; specialization is a config override or a wrapper that delegates to the base through the `HarnessClass` port declared by the use-case crate. No subclassing, no concrete-detail import in the inner circle (clean-architecture: dependency arrow stays inward; LSP preserved).
3. **Router is a pure function.** `fn select(&[ExecutionRecord], &Task) -> HarnessConfig` — no stateful router, no mutation. Routing lessons are immutable `ExecutionRecord` slices passed in. Never select a class that regressed on the task's cluster.
4. **Lean default.** The base config keeps the minimal tool set and tightest budget; specialization adds capability only when execution-record evidence shows benefit (per the paper's ablation finding that uniform capability hurts easy tasks).
5. **Pipeline shape.** `input → classify_task → select_config → run_harness → record_outcome`; each stage pure, effects (tool calls, model requests) pushed to the edges behind ports.
6. **Integration with team mode.** Workers in M8 already carry a strategy; the `HarnessConfig` is the per-worker execution strategy. No new store or port required — the existing `Strategy` enum becomes the config key.

## Consequences

Easier: per-task optimization without risking easy-task regressions; a principled place to put strategy-specific context budgets and tool subsets; aligns with the existing Strategy/Worker model.

Harder: router quality depends on execution-record volume; cold-start routing defaults to `build`; adds a small indirection in the loop.

Not doing: no new storage, no new port, no PQC, no change to M1–M8 gates. This is an optimization layer, not a new milestone.

## Alternatives considered

- **Keep one global harness with strategy flags** (status quo): rejected because the ablation evidence shows uniform capability hurts easy tasks.
- **Separate agent per strategy** (role-based): rejected — strategies are use cases, not agent types (invariant 16).
- **Meta-Harness global evolution**: rejected in favor of OO-Harness because global evolution showed memory additions degrading easy tasks; specialization isolates the regression.

## References

- Li et al., "Object-Oriented Harness: Evolving Agent Harnesses through Object-Oriented Design," arXiv:2610.28380, 2026.
- Orexis ADR-0013 (entity model), ADR-0020 (team-mode domain).
