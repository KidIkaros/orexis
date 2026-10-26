# MUE-X — study

**Source:** `github.com/KorroAi/mue-x` · **Language:** Python · **Size:** moderate · **License:** MIT · **Last seen:** upstream main

**Why it is in the study:** it is the only source with a working self-evolution loop — mutation strategies, fitness, rollback, an immune system, drives, PAD affect, and a multi-layer memory lattice. The ideas are strong; the implementation quality is uneven.

## Known surface

| Area | What it does |
|---|---|
| Mutation | Six AST strategies: explore, exploit, repair, optimize, innovate, prune |
| Safety | Gene backups, validation, rollback, deduplication, fitness scoring, size limits, mitosis |
| Autonomy | Autonomous signals and seven drives |
| Affect | PAD-style emotional model |
| Memory | Six-layer SQLite FTS5 lattice |
| Absorption | GitHub repository absorption |
| Optimization | RL bandit optimizer |
| Orchestration | Swarm orchestration |
| Guard | Security guard and protected files |

## Dimensions to fill

1. **Process model** — always-on loop or triggered cycle? How does the swarm run?
2. **Turn loop** — separate from the evolution loop?
3. **Session model** — what is persisted per cycle?
4. **Tool protocol** — how tools are described and called.
5. **Context management** — what the memory lattice injects.
6. **Provider abstraction** — provider support and fallbacks.
7. **Entity substrate** — seven drives, PAD; how they steer behavior.
8. **Evolution** — the load-bearing section: mutation, fitness, rollback, immune.
9. **Interop** — none significant.
10. **Surfaces** — CLI.
11. **Quality practice** — tests and what they miss.
12. **Failure modes** — hobby-grade implementation; unsafe autonomy.

## Known evidence

- Six mutation strategies are a good vocabulary for gene-level evolution.
- The immune system, protected files, and rollback are the safety model Orexis must keep.
- The six-layer lattice is a memory design worth comparing with Kindred's.
- PAD affect is the input to Orexis's affect model, alongside Kindred's glial layer.
- The always-on autonomy loop is rejected: Orexis evolves on cron/manual triggers, visibly.

## Adopt

- **Mutation strategies as data genes** (explore, exploit, repair, optimize, innovate, prune).
- **Immune safeguards**: protected files, validation, rollback, dedup, size limits.
- **Fitness scoring** over execution traces.
- **Six-layer memory lattice** as a comparison point with Kindred.
- **PAD affect** as an input to routing and risk.

## Adapt

- Mutation to a **data-only gene pool**; Python/Rust code changes require approval.
- Shared pools only through **signed releases**, never live code.
- Per-profile evolution by default; optional shared pool through sanitized PRs.

## Reject

- Always-on hidden autonomy loops.
- AST mutation of the runtime without approval.
- GitHub absorption in v1.
- The implementation's test/quality level.

## Open questions

- How is fitness computed, and over what window?
- What exactly triggers a rollback, and how is a bad mutation detected?
- How do the six memory layers differ in query and decay?
- How do the seven drives map onto Kindred's five?
