# Synthesis — eight studies into one architecture

This document collapses the per-repo studies into decisions. It is the input to `docs/orexis/adr/` and to the map's *Decisions so far*.

## Status

Not started. The per-repo documents are stubs with known evidence; each must be filled before this synthesis is written.

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

| Conflict | Options | Where it resolves |
|---|---|---|
| Plugin tree vs narrow waist | Cordis-style unprivileged core vs ports + seams | ADR; v1 is ports + seams |
| Event log vs message store | Event-sourced vs message rows | Session ADR (frontier #1) |
| Entity-first vs tool-first | Entity as domain vs harness as product | Plan §0, §1.5: harness first, entity as position |
| Cache stability vs dynamic context | Fixed prefix vs mid-session updates | Invariant 3; compression is the only break |
| Code-mode vs native calls | Program surface vs tool calls | Code-mode opt-in, native default |
| Single-agent vs MAS | Serial default vs parallel team | Invariant 19; team opt-in |
| MCP vs UTCP | Server proxy vs direct transports | MCP first; UTCP ported later |
| ACP vs A2A | Editor/host vs mesh | ACP in v1; A2A in M7 |

## Porting ledger

See `docs/orexis/PLAN.md` §6. It is one-way: a module is ported only if it passes the spec, with its tests.

## ADRs to write

1. Dependency Rule and crate graph.
2. Async ports and the runtime boundary.
3. Session event model and versioning.
4. Provider catalog and profile schema.
5. Composition root.
6. Evolution safety boundary.
7. Hermes interoperability boundary (MCP/ACP).
8. TUI/desktop surface contract.
9. Team-mode gate and concurrency.
10. Entity model.

## Open questions

- Which of Kindred's five drives survive the study, and does growth become the sixth?
- Does the six-layer lattice replace or wrap Kindred's memory?
- What does the team benchmark measure, exactly, and on which tasks?
- Which provider profiles need native adapters at M1 versus M7?
