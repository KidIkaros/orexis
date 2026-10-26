# Synthesis — eight studies into one architecture

This document collapses the per-repo studies into decisions. It is the input to `docs/orexis/adr/` and to the map's *Decisions so far*.

## Status

**Drafted 2026-10-25.** The eight per-repo documents are filled to the adopt/adapt/reject level. Claims are cited to a file, a line, or the eleven-system corpus study (arXiv 2609.00006). Items marked *(secondary)* come from the corpus study or a README rather than a direct source read; they are sufficient for the decision but should be re-verified during the port.

**Confidence:** high for Kindred, Harness, and Hermes (local source read); medium for DeepSeek Harness and opencode (corpus study + docs); medium for MUE-X and Mercury (README/module-level). Familiar is a failure analysis from its own design documentation.

## Pattern catalog

| Pattern | Source(s) | Decision | Lands in |
|---|---|---|---|
| Client/server harness; every UI a client | opencode, OpenHands, DeepSeek | Adopt | `orexis serve` + TUI/desktop |
| Event-sourced sessions with replay | OpenHands, DeepSeek, opencode | Adopt | `usecases::session`, `session-sqlite` |
| Versioned session generations + adjacent migrations | DeepSeek | Adopt | `session-sqlite` |
| Model-visible means logged | DeepSeek | Adopt | `usecases::session` |
| Capability seams (definition + provider + consumer) | DeepSeek | Adopt | `orexis-ports` |
| Provider profiles as data | Hermes (43) | Adopt | `adapters-provider-*` |
| Model-family prompt matrix + cache-dialect fanout | opencode, Codex | Adopt | `adapters-provider-*` |
| Skills as capability bundles | Hermes, opencode, 9/11 corpus | Adopt | `usecases::skills` |
| Skill trust tiers + quarantine | Hermes | Adopt | `usecases::permissions` |
| ACP server | 6/11 corpus | Adopt | `adapters-protocol-acp` |
| MCP client | Hermes, Kindred, opencode | Adopt | `adapters-protocol-mcp` |
| Policy-as-code with a floor under YOLO | Hermes, Codex | Adopt | `usecases::permissions` |
| Untrusted-content delimiting | Hermes, OpenHands | Adopt | `usecases::tools` |
| Turn-level shadow-git checkpoints | opencode, Hermes | Adopt | `usecases::session` |
| Session-tree version control | Pi, OpenHands | Adopt | `usecases::session` |
| Threshold compaction with verbatim tail | Claude Code, Gemini, Hermes | Adopt | `usecases::context` |
| Lineage compaction | Hermes | Adopt | `usecases::session` |
| Deferred tool loading above ~15 tools | Claude Code, Codex, Hermes | Adopt | `usecases::tools` |
| Exact + fuzzy edit cascade | Claude Code, opencode, Hermes | Adopt | `usecases::tools` |
| Syntax-aware command permissioning | opencode, Hermes | Adopt | `usecases::permissions` |
| Verify-on-stop / outer verification | OpenHands, Hermes | Adopt | `usecases::strategy` |
| Daily token budget + cheap-model routing | Mercury | Adopt | `usecases::budget` |
| Typed memory with confidence/conflicts | Mercury | Adopt | `usecases::memory` |
| Six-layer memory lattice | MUE-X | Adapt | `usecases::memory` |
| Drives + glial modulation + pulse | Kindred | Adapt | `usecases::entity` |
| PAD affect | MUE-X | Adapt | `usecases::entity` |
| Mutation strategies + immune + rollback | MUE-X | Adapt | `usecases::evolution` |
| Genes corridor + seal | Kindred, Harness | Adapt | `usecases::evolution` |
| Everything-is-a-plugin (Cordis) | DeepSeek | Reject for v1 | v2 direction |
| Agentic frameworks | none in production | Reject | — |
| Vector RAG over code | none in production | Reject | — |
| Line-number edits | none in production | Reject | — |
| Harness identity mimicry | Pi | Reject | — |
| Always-on evolution loop | MUE-X | Reject | cron/manual only |
| Relationship/companion features | Kindred | Reject | — |
| Web surface | — | Reject | desktop only |

## Conflicts to resolve

| Conflict | Options | Resolved as |
|---|---|---|
| Plugin tree vs narrow waist | Cordis-style unprivileged core vs ports + seams | **Ports + seams** in v1; the plugin tree is a v2 direction (ADR-0002, plan §4.1) |
| Event log vs message store | Event-sourced vs message rows | **Event-sourced** with versioned generations (ADR-0001) |
| Entity-first vs tool-first | Entity as domain vs harness as product | **Harness first, entity as position**; M1–M3 ship a trustworthy harness before M4–M6 (plan §0, §1.5) |
| Cache stability vs dynamic context | Fixed prefix vs mid-session updates | **Fixed prefix**; compression is the only sanctioned break (invariant 3) |
| Code-mode vs native calls | Program surface vs tool calls | **Native default, code-mode opt-in** as a program surface (ADR-0003) |
| Single-agent vs MAS | Serial default vs parallel team | **Serial default, team opt-in** with a benchmark gate (invariant 19, ticket 008) |
| MCP vs UTCP | Server proxy vs direct transports | **MCP first** (49k servers, 19:1 SDK downloads); UTCP ported in M7 |
| ACP vs A2A | Editor/host vs mesh | **ACP server in v1** (6/11 harnesses); A2A in M7 (1/11) |
| Five vs six drives | Kindred code vs its design doc | **Deferred to ticket 007**; growth is not assumed |
| Exact vs fuzzy edits | Frontier vs weaker models | **Both**, selected by model tier (ADR pending, ticket 005) |

## Porting ledger

See `docs/orexis/PLAN.md` §6. It is one-way: a module is ported only if it passes the spec, with its tests.

## ADRs

| ADR | Status |
|---|---|
| 0001 — Session event model and versioning | accepted |
| 0002 — Provider catalog and profile | accepted |
| 0003 — Codec set and selection | accepted |
| 0004 — TUI surface contract | accepted |
| Dependency Rule and crate graph | pending (M1) |
| Async ports and the runtime boundary | pending (M1) |
| Composition root | pending (M1) |
| Entity model | pending (ticket 007) |
| Evolution safety boundary | pending (ticket 008) |
| Hermes interoperability boundary (MCP/ACP) | pending (M7) |
| Team-mode gate and concurrency | pending (M8) |
| Desktop surface contract | pending (ticket 006) |

## Open questions

- Which of Kindred's five drives survive, and does growth become the sixth? *(ticket 007)*
- Does the six-layer lattice replace or wrap Kindred's memory?
- What does the team benchmark measure, exactly, and on which tasks? *(plan §9 gate)*
- Which provider profiles need native adapters at M1 versus M7? *(ticket 009)*
- Do DeepSeek's session-generation and Hermes's `state.db` agree on migration semantics? *(verify during ADR-0001 implementation)*
