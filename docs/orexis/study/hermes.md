# Hermes — study

**Source:** `github.com/NousResearch/hermes-agent` · **Language:** Python · **Size:** ~263k production LOC, ~334k test LOC · **License:** MIT · **Last seen:** local `v0.22.5` plus upstream main

**Why it is in the study:** it is the breadth reference — 43 provider profiles, a stable plugin API, 210 skills, 38 gateway adapters, ACP, MCP, and two invariants Orexis adopts outright: prompt caching is sacred, and the core is a narrow waist.

## Known surface

| Area | What it does |
|---|---|
| Providers | 43 `ProviderProfile`s under `plugins/model-providers/`; lazy discovery; last-writer-wins; OAuth and key auth |
| Plugins | Tools, commands, memory providers, context engines, hooks, approval transports, delegation providers, prompt sections, locales, platforms |
| Skills | SKILL.md bundles; curator; self-improving skill loop; trust tiers and quarantine |
| Sessions | `state.db` with system prompts, messages, sessions, generations; schema versions; lineage compaction |
| Safety | Policy floor that survives `--yolo`; twelve hardline patterns; untrusted-content delimiting |
| Delegation | `max_concurrent_children` 10, `max_spawn_depth` 1, `max_live_sessions` 16 |
| Interop | ACP adapter; MCP client; gateway platforms |
| Surfaces | CLI, TUI, desktop, gateway, dashboard |

## Dimensions to fill

1. **Process model** — CLI, gateway, TUI, desktop; one core, many surfaces; profile scope.
2. **Turn loop** — budgeted loop with stop-guards; turn phases.
3. **Session model** — `state.db` schema; generations; lineage compaction; replay.
4. **Tool protocol** — native tool calls; XML fallback?; toolset gating; deferred loading.
5. **Context management** — three-tier assembly with bit-perfect prefixes; compression.
6. **Provider abstraction** — `ProviderProfile` fields; OAuth vs key; cache dialects.
7. **Entity substrate** — personality, memory, journey; no drives/affect.
8. **Evolution** — skill curator; self-improving loop; no gene mutation.
9. **Interop** — ACP server; MCP client; A2A?; gateway adapters.
10. **Surfaces** — CLI, TUI, desktop, gateway; what owns state.
11. **Quality practice** — 334k test LOC; generated docs; plugin compat contract.
12. **Failure modes** — 263k LOC; 460 commits/day; god-file history; fork drift.

## Known evidence

- `ProviderProfile` is the pattern for Orexis's provider catalog: data, lazy discovery, user override.
- The prompt-caching invariant (no mid-conversation prefix mutation except compression) is adopted.
- The policy floor under `--yolo` is the safety pattern Orexis adopts.
- Untrusted-content delimiting and lineage compaction are directly portable.
- The plugin API's behavior contract (additive hooks, signature inspection, no re-export shims) is a lesson in stability.
- A fork is rejected: 263k LOC and 460 commits/day of upstream drift.

## Adopt

- **ProviderProfile** as data, with a broad shipped catalog.
- **Prompt-caching invariant.**
- **Policy floor under YOLO.**
- **Untrusted-content delimiting.**
- **Lineage compaction.**
- **Skills + trust tiers + quarantine.**
- **ACP server.**
- **Plugin compatibility as a behavior contract**, not a version literal.

## Adapt

- The 43 profiles into Orexis's catalog (many collapse into the compatible adapter).
- The plugin API into Orexis's seam model, without porting the Python tree.

## Reject

- Forking the codebase.
- Python as the implementation language.
- God-file facade/sibling layout as a template.
- Gateway breadth in v1.

## Open questions

- What exactly does a `ProviderProfile` carry, and how are OAuth profiles refreshed?
- How does `state.db` version and migrate without breaking replay?
- How does the curator decide a skill is worth keeping?
- Where does the prompt prefix get rebuilt, and what triggers it?
