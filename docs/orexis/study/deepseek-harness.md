# DeepSeek Harness — study

**Source:** `github.com/deepseek-ai/deepseek-harness` · **Language:** TypeScript · **Size:** large monorepo · **License:** MIT · **Last seen:** upstream main

**Why it is in the study:** it is the clearest statement of the architectural invariants Orexis adopts — capability seams, three event domains, "model-visible means logged," versioned session formats, and fresh-process programmatic tool calling.

## Known surface

| Area | What it does |
|---|---|
| Plugin model | Everything is a plugin on Cordis; profiles → bundles → patch layers; no privileged core |
| Seams | A capability = service definition + provider + consumer; reversible effects |
| Events | Three domains: durable session events, live agent events, capability/policy events |
| Session log | Versioned, immutable generations with adjacent migrations; replay reconstructs every model request |
| PTC | Programmatic tool calling defaults to fresh processes; persistent REPL is opt-in |
| Docs | Generated config catalogs, capability graphs, contracts; `SAFETY.md` separate from security policy |
| Agent teams | Experimental package: `maxMembers` 16, `maxTasks` 256, versioned task snapshots, `expectedRevision`, write scopes |

## Dimensions to fill

1. **Process model** — Cordis plugin tree; how services are resolved; how a profile composes bundles.
2. **Turn loop** — where the loop lives when everything is a plugin.
3. **Session model** — generation format, migrations, replay, fork semantics.
4. **Tool protocol** — native calls, PTC, and how a program's tool calls are admitted.
5. **Context management** — what is assembled, what is logged, what is compacted.
6. **Provider abstraction** — adapters as plugins; model catalog.
7. **Entity substrate** — absent by design? Any persona/memory primitives?
8. **Evolution** — any self-modification, or only configuration?
9. **Interop** — MCP, ACP, A2A support.
10. **Surfaces** — CLI, SDK, server.
11. **Quality practice** — generated contracts, tests, conformance.
12. **Failure modes** — what the plugin model costs in debuggability.

## Known evidence

- The three event domains and the logged invariant are directly portable.
- Session generations are immutable; migrations sit adjacent to the format.
- Agent teams are bounded and versioned; concurrency across processes is explicitly unsupported.
- The plugin tree is powerful and expensive; Orexis defers it to v2.

## Adopt

- **Model-visible means logged**, with replay as a pure projection.
- **Versioned session formats** with adjacent migrations and immutable generations.
- **Capability seams**: definition + provider + consumer, reversible effects.
- **Fresh-process PTC default**; persistent REPL opt-in.
- **Generated contracts** and a `SAFETY.md` distinct from the security policy.
- **Agent-team limits**: 16 members, 256 tasks, versioned snapshots.

## Adapt

- The seam vocabulary, without the full Cordis plugin tree in v1.
- Profiles and bundles as a v2 direction, not a v1 requirement.

## Reject

- A fully unprivileged plugin core in v1 — too much complexity before the loop is proven.

## Open questions

- How does replay reconstruct a request byte-for-byte across provider changes?
- What exactly is in a session generation, and how are migrations tested?
- How does the agent-team board handle stale revisions under concurrent writers?
